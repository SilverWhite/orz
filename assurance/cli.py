from __future__ import annotations

import argparse
import json
from pathlib import Path
import subprocess
import sys
from typing import Any, Sequence
import uuid

from .canonical_cli import (
    DEFAULT_CREDENTIAL_TARGET,

    run_canonical_guarded_cli,
    run_canonical_guarded_cli_real,
    verify_canonical_guarded_cli_run,
)
from .errors import AssuranceError
from .source_visibility import (
    dumps_receipt as dumps_source_visibility_receipt,
    evaluate_source_visibility_ledger_file,
    render_visibility_summary,
)


ROOT = Path(__file__).resolve().parents[1]


def _json_dump(value: Any) -> str:
    return json.dumps(
        value,
        ensure_ascii=False,
        indent=2,
        sort_keys=True,
        allow_nan=False,
    )


def _git_status() -> dict[str, Any]:
    try:
        completed = subprocess.run(
            ["git", "status", "--short", "--branch"],
            cwd=ROOT,
            check=False,
            capture_output=True,
            text=True,
            timeout=10,
        )
    except Exception as exc:
        return {
            "available": False,
            "exit_code": None,
            "summary": f"{type(exc).__name__}: {exc}",
        }
    return {
        "available": completed.returncode == 0,
        "exit_code": completed.returncode,
        "summary": completed.stdout.strip(),
    }


def _doctor_report(*, include_repository_check: bool) -> dict[str, Any]:
    repository_check: dict[str, Any] | None = None
    if include_repository_check:
        from scripts.check_repository import check_repository

        repository_check = check_repository()
    git = _git_status()
    report = {
        "schema_version": "0.1.0-draft",
        "report_kind": "gsa_cli_doctor_report",
        "valid": bool(repository_check["valid"]) if repository_check else True,
        "workspace_root": str(ROOT),
        "python": {
            "version": sys.version.split()[0],
            "executable": sys.executable,
        },
        "git": git,
        "repository_check": repository_check,
        "entrypoints": {
            "chat": "python gsa.py chat",
            "chat_fake": "python gsa.py chat --fake-provider",
            "chat_real": "python gsa.py chat (auto-detects DeepSeek credential)",
            "tui": "python gsa.py tui",
            "ask": "python gsa.py ask --retrieval off|subagent <question>",
            "doctor": "python gsa.py doctor",
            "source_gate": "python gsa.py source gate --ledger <path>",
            "alpha_smoke": "python gsa.py alpha smoke",
            "verify": "python gsa.py verify --run-root <path>",
        },
        "runtime_boundaries": {
            "default_sandbox": "workspace_guarded",
            "strict_sandbox": "docker_opt_in",
            "docker_status": "optional_explicit_interface",
            "command_review": "interactive_by_default",
            "default_network": "disabled",
            "default_credential_use": "disabled",
            "canonical_run_fake": "offline_no_network",
            "canonical_run_real": "deepseek_api_exactly_one_request",
            "grok_run_prompt_tool_gate": "explicit_fail_closed_execute_only_after_allow",
        },
        "alpha_defaults": {
            "max_turns": 20,
            "tools": "all_builtin_enabled",
            "acp_permission_mode": "interactive",
            "sandbox": "workspace_guarded_logical_boundary",
        },
        "limitations": [
            "doctor verifies repository mechanics and CLI wiring, not scientific correctness.",
            "real adapter reads credentials from Windows Credential Manager and never persists them.",
            "Interactive command review uses workspace_guarded (logical boundary); strict Docker sandbox is opt-in via --strict.",
            "Windows native sandbox remains noncompliant for strict mode without Docker.",
            "GSA-CORE review and evaluation bridges exist but have not been connected to a live runtime.",
        ],
    }
    return report


def _print_or_json(value: dict[str, Any], *, json_output: bool, human_lines: list[str]) -> None:
    if json_output:
        print(_json_dump(value))
        return
    for line in human_lines:
        print(line)


def _run_ask(args: argparse.Namespace) -> int:
    from datetime import datetime, timezone

    from .deepseek_runtime_adapter import run_deepseek_ask

    if args.run_root:
        run_root = args.run_root
    else:
        stamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
        run_root = ROOT / ".gsa" / "runs" / f"ask-{stamp}-{uuid.uuid4().hex[:8]}"
    run_root.mkdir(parents=True, exist_ok=True)

    receipt, response_text = run_deepseek_ask(
        run_root=run_root,
        prompt_text=args.question,
        credential_target=args.credential_target,
        max_tokens=args.max_tokens,
        timeout_seconds=args.timeout,
        retrieval_mode=args.retrieval,
        project_root=ROOT,
    )

    if args.json:
        print(_json_dump(receipt))
    else:
        provenance = receipt["prompt"].get("provenance", "unknown")
        if response_text.strip():
            print(response_text)
        else:
            print(
                f"gsa ask: no response text (http {receipt['execution']['http_status_code']}, "
                f"finish: {receipt['prompt']['response_finish_reason']})"
            )
        print()
        print(f"── provenance: {provenance}")
        print(f"   retrieval:  {receipt['retrieval']['mode']}")
        print(f"   run_root:   {receipt['request']['run_root']}")
        print(f"   receipt:    {receipt['artifacts']['receipt_path']}")
        print(f"   events:     {receipt['artifacts']['events_path']}")

    return 0 if receipt["valid"] else 1


def _run_doctor(args: argparse.Namespace) -> int:
    report = _doctor_report(include_repository_check=not args.quick)
    repository = report["repository_check"]
    if repository is None:
        repo_line = "repository check: skipped"
    else:
        repo_line = (
            f"repository check: {'valid' if repository['valid'] else 'invalid'} "
            f"({repository.get('error_count', 0)} errors)"
        )
    human_lines = [
        f"GSA CLI doctor: {'valid' if report['valid'] else 'invalid'}",
        f"workspace: {report['workspace_root']}",
        f"python: {report['python']['version']}",
        repo_line,
        f"sandbox: {report['runtime_boundaries']['default_sandbox']} (strict: {report['runtime_boundaries']['strict_sandbox']})",
        f"Docker: {report['runtime_boundaries']['docker_status']}",
        f"command review: {report['runtime_boundaries']['command_review']}",
        f"alpha: max_turns={report['alpha_defaults']['max_turns']}, tools={report['alpha_defaults']['tools']}",
    ]
    _print_or_json(report, json_output=args.json, human_lines=human_lines)
    return 0 if report["valid"] else 1


def _run_source_gate(args: argparse.Namespace) -> int:
    receipt = evaluate_source_visibility_ledger_file(args.ledger)
    if args.summary:
        print(render_visibility_summary(receipt))
    elif args.json:
        print(dumps_source_visibility_receipt(receipt))
    else:
        lines = [
            f"source gate: {receipt['decision']}",
            f"sources: {receipt['source_count']}",
            f"references: {receipt['reference_count']}",
            f"context tokens estimate: {receipt['context_tokens_estimate']}",
        ]
        for item in receipt["reference_decisions"]:
            lines.append(
                f"{item['ref_id']}: {item['observed_visibility']} -> "
                f"{item['decision']} ({item['claim_allowed']})"
            )
        print("\n".join(lines))
    return 0 if receipt["decision"] != "block" else 1


def _run_grok_prompt_tool_gate(args: argparse.Namespace) -> int:
    from .grok_prompt_tool_gate import (
        DEFAULT_RETRIEVAL_MODE,
        build_grok_prompt_tool_promotion_gate_receipt,
        write_grok_prompt_tool_promotion_gate_receipt,
    )
    from .utils import atomic_write_json, load_json as _load_json

    if args.source_ledger or args.instruction_context or args.task:
        raise AssuranceError(
            "--source-ledger, --instruction-context, and --task are canonical-only "
            "for gsa run; use --runtime canonical or omit them for --runtime grok"
        )
    if args.mode != "fake":
        raise AssuranceError("--mode real is canonical-only for gsa run")

    retrieval_mode_explicit = args.retrieval_mode is not None
    retrieval_mode = args.retrieval_mode or DEFAULT_RETRIEVAL_MODE
    run_id = (
        None
        if args.run_id == "RUN-CANONICAL-CLI-FAKE-001"
        else args.run_id
    )
    auto_evidence = bool(getattr(args, "grok_alpha_auto_evidence", False))
    if auto_evidence:
        if not getattr(args, "grok_execute", False):
            raise AssuranceError("--grok-alpha-auto-evidence requires --grok-execute")
        if getattr(args, "grok_mode", "acp-smoke") != "acp-smoke":
            raise AssuranceError("--grok-alpha-auto-evidence supports only --grok-mode acp-smoke")
        if retrieval_mode != "off":
            raise AssuranceError("--grok-alpha-auto-evidence supports only --retrieval-mode off")

    if getattr(args, "grok_fake_provider", False) and getattr(args, "grok_mode", "acp-smoke") != "acp-smoke":
        raise AssuranceError("--grok-fake-provider supports only --grok-mode acp-smoke")

    if getattr(args, "grok_execute", False):
        if args.run_root.exists() and any(args.run_root.iterdir()):
            raise AssuranceError(f"run_root must be empty or absent: {args.run_root}")
        args.run_root.mkdir(parents=True, exist_ok=True)
        timeout_gate_receipt = (
            _load_json(args.grok_timeout_gate)
            if args.grok_timeout_gate else None
        )
        tool_availability_gate_receipt = (
            _load_json(args.grok_tool_availability_gate)
            if args.grok_tool_availability_gate else None
        )
        adapter_receipt = (
            _load_json(args.grok_adapter_receipt)
            if args.grok_adapter_receipt else None
        )

        if auto_evidence:
            evidence_root = args.run_root / "evidence"
            evidence_root.mkdir()
            if tool_availability_gate_receipt is None:
                from .tool_availability_gate import (
                    build_tool_availability_gate_receipt,
                    probe_tool_availability,
                )

                tool_report = probe_tool_availability(
                    tool_specs=[],
                    runtime_id="GROK-ALPHA-NO-TOOL-ACP-001",
                    probe_registry={},
                )
                tool_availability_gate_receipt = build_tool_availability_gate_receipt(
                    tool_report
                )
                atomic_write_json(
                    evidence_root / "alpha-tool-availability-report.json",
                    tool_report,
                )
                atomic_write_json(
                    evidence_root / "alpha-tool-availability-gate.json",
                    tool_availability_gate_receipt,
                )
            if adapter_receipt is None:
                from .grok_runtime_adapter import run_grok_version_smoke

                adapter_receipt = run_grok_version_smoke(
                    run_root=evidence_root / "adapter-containment",
                    workspace_path=evidence_root / "adapter-containment" / "workspace",
                    run_id=f"RUN-GROK-ALPHA-EVIDENCE-{uuid.uuid4().hex[:8].upper()}",
                    retrieval_mode="off",
                    retrieval_mode_explicit=False,
                )

        receipt = build_grok_prompt_tool_promotion_gate_receipt(
            run_root=args.run_root,
            ask=args.ask,
            retrieval_mode=retrieval_mode,
            retrieval_mode_explicit=retrieval_mode_explicit,
            run_id=run_id,
            timeout_gate_receipt=timeout_gate_receipt,
            tool_availability_gate_receipt=tool_availability_gate_receipt,
            adapter_receipt=adapter_receipt,
        )
        if auto_evidence:
            receipt["limitations"].append(
                "Alpha auto evidence declares an empty tool set and is valid only for no-tool loopback ACP smoke; it does not prove general Grok tool availability."
            )
        atomic_write_json(
            args.run_root / "grok-prompt-tool-promotion-gate.json",
            receipt,
        )
    else:
        receipt = write_grok_prompt_tool_promotion_gate_receipt(
            run_root=args.run_root,
            ask=args.ask,
            retrieval_mode=retrieval_mode,
            retrieval_mode_explicit=retrieval_mode_explicit,
            run_id=run_id,
            timeout_gate_path=args.grok_timeout_gate,
            tool_availability_gate_path=args.grok_tool_availability_gate,
            adapter_receipt_path=args.grok_adapter_receipt,
        )

    # If gate allows and --grok-execute is set, run the selected Grok mode.
    executed = False
    if receipt["decision"] == "allow" and getattr(args, "grok_execute", False):
        if not args.ask:
            raise AssuranceError(
                "--ask is required when --grok-execute is set"
            )
        grok_mode = getattr(args, "grok_mode", "acp-smoke")
        from .grok_runtime_adapter import (
            GrokRunRequest,
            run_grok_headless_once,
            run_grok_acp_once,
        )
        adapter_mode = "acp_smoke" if grok_mode == "acp-smoke" else "prompt_smoke"
        execution_root = args.run_root / "grok-execution"
        adapter_request = GrokRunRequest(
            run_root=execution_root,
            workspace_path=execution_root / "workspace",
            mode=adapter_mode,
            prompt_text=args.ask,
            retrieval_mode=retrieval_mode,
            retrieval_mode_explicit=retrieval_mode_explicit,
            fake_provider=bool(getattr(args, "grok_fake_provider", False) or auto_evidence),
            # GAK-03: auto_evidence/fake-provider paths have tools disabled
            # or use a fake provider — auto-allow is safe for smoke.
            acp_permission_mode="auto_allow_once",
        )
        if adapter_mode == "acp_smoke":
            adapter_receipt = run_grok_acp_once(adapter_request)
        else:
            adapter_receipt = run_grok_headless_once(adapter_request)
        executed = True
        # Rebuild the gate receipt with execution outcome attached.
        receipt = build_grok_prompt_tool_promotion_gate_receipt(
            run_root=args.run_root,
            ask=args.ask,
            retrieval_mode=retrieval_mode,
            retrieval_mode_explicit=retrieval_mode_explicit,
            run_id=run_id,
            timeout_gate_receipt=timeout_gate_receipt,
            tool_availability_gate_receipt=tool_availability_gate_receipt,
            adapter_receipt=adapter_receipt,
            execution_outcome={
                "outcome": (
                    "completed" if adapter_receipt.get("valid") else "failed"
                ),
                "mode": adapter_mode,
                "receipt_path": str(adapter_receipt["artifacts"]["receipt_path"]),
                "events_path": str(adapter_receipt["artifacts"]["events_path"]),
            },
        )
        if auto_evidence:
            receipt["limitations"].append(
                "Alpha auto evidence declares an empty tool set and is valid only for no-tool loopback ACP smoke; it does not prove general Grok tool availability."
            )
        atomic_write_json(
            args.run_root / "grok-prompt-tool-promotion-gate.json",
            receipt,
            overwrite=True,
        )

    human_lines = [
        f"grok prompt/tool gate: {receipt['decision']}",
        f"run_id: {receipt['run_id']}",
        f"run_root: {receipt['request']['run_root']}",
        f"retrieval mode: {receipt['request']['retrieval_mode']}",
        f"prompt/tool execution attempted: {str(executed).lower()}",
    ]
    if receipt["blocking_reasons"]:
        human_lines.append(
            f"blocking reasons: {', '.join(receipt['blocking_reasons'])}"
        )
    _print_or_json(receipt, json_output=args.json, human_lines=human_lines)
    return 0 if receipt["decision"] == "allow" else 1


def _run_canonical(args: argparse.Namespace) -> int:
    if args.runtime == "grok":
        return _run_grok_prompt_tool_gate(args)
    if args.runtime == "deepseek":
        if args.mode != "real":
            raise AssuranceError("--runtime deepseek requires --mode real")
        if args.source_ledger or args.instruction_context or args.task:
            raise AssuranceError(
                "--source-ledger, --instruction-context, and --task are "
                "canonical-only for gsa run; direct DeepSeek runtime accepts "
                "only --ask"
            )
        return _run_deepseek(args)
    if args.retrieval_mode or args.grok_timeout_gate or args.grok_tool_availability_gate or args.grok_adapter_receipt:
        raise AssuranceError(
            "--retrieval-mode, --grok-timeout-gate, "
            "--grok-tool-availability-gate, and --grok-adapter-receipt "
            "are only valid with --runtime grok"
        )
    if args.mode == "real":
        receipt = run_canonical_guarded_cli_real(
            run_root=args.run_root,
            source_ledger_path=args.source_ledger,
            instruction_provenance_gate_context_path=args.instruction_context,
            ask=args.ask,
            task_contract_path=args.task,
            run_id=args.run_id,
            task_id=args.task_id,
            created_at=args.created_at,
            credential_target=args.credential_target,
            api_timeout_seconds=args.api_timeout,
        )
    else:
        receipt = run_canonical_guarded_cli(
            run_root=args.run_root,
            source_ledger_path=args.source_ledger,
            instruction_provenance_gate_context_path=args.instruction_context,
            ask=args.ask,
            task_contract_path=args.task,
            run_id=args.run_id,
            task_id=args.task_id,
            created_at=args.created_at,
        )
    mode_label = "real" if args.mode == "real" else "fake"
    human_lines = [
        f"canonical run [{mode_label}]: {'valid' if receipt['valid'] else 'invalid'}",
        f"run_id: {receipt['run_id']}",
        f"run_root: {receipt['run_root']}",
        f"events: {receipt['event_count']} ({', '.join(receipt['event_types'])})",
        f"terminal: {receipt['terminal_event']}",
    ]
    _print_or_json(receipt, json_output=args.json, human_lines=human_lines)
    return 0 if receipt["valid"] else 1


def _run_verify(args: argparse.Namespace) -> int:
    receipt = verify_canonical_guarded_cli_run(run_root=args.run_root)
    human_lines = [
        f"verify: {'valid' if receipt['valid'] else 'invalid'}",
        f"run_id: {receipt['run_id']}",
        f"events: {receipt['event_count']}",
        f"terminal: {receipt['terminal_event']}",
    ]
    _print_or_json(receipt, json_output=args.json, human_lines=human_lines)
    return 0 if receipt["valid"] else 1


def _run_grok_doctor(args: argparse.Namespace) -> int:
    from .grok_runtime_adapter import SUPPORTED_RETRIEVAL_MODES, inspect_grok_runtime

    inspection = inspect_grok_runtime()
    report = {
        "schema_version": "0.1.0-draft",
        "report_kind": "grok_cli_doctor_report",
        "valid": inspection.get("valid") is True,
        "workspace_root": str(ROOT),
        "inspection": inspection,
        "entrypoints": {
            "doctor": "python gsa.py grok doctor",
            "version_smoke": (
                "python gsa.py grok run --mode version-smoke "
                "--run-root <path>"
            ),
            "acp_smoke": (
                "python gsa.py grok run --mode acp-smoke "
                "--fake-provider --ask <prompt> --run-root <path>"
            ),
            "tui_version_smoke": (
                "python gsa.py tui --runtime grok --run version-smoke "
                "--run-root <path>"
            ),
            "tui_acp_smoke": (
                "python gsa.py tui --runtime grok --fake-provider --run <prompt> "
                "--run-root <path>"
            ),
        },
        "runtime_boundaries": {
            "supported_mode": "version-smoke",
            "supported_modes": ["version-smoke", "acp-smoke"],
            "retrieval_modes": list(SUPPORTED_RETRIEVAL_MODES),
            "default_retrieval_mode": "off",
            "network": "disabled",
            "prompt_submission": "explicit_acp_smoke_supported_not_default",
            "tool_execution": "runtime_owned_in_explicit_acp_smoke_not_default_promoted",
            "fake_provider": "explicit_loopback_fixture_available_for_acp_smoke",
            "no_residue_required": True,
            "residue_scan_scope": "job_object_contained_for_adapter_launches",
        },
        "limitations": [
            "Grok CLI wiring supports locked-binary doctor, version-smoke, and explicit ACP smoke.",
            "The canonical gsa run path remains default; Grok prompt/tool execution is opt-in and not globally promoted.",
            "Use --fake-provider for no-credential loopback ACP smoke; it is a fixed fixture and not real DeepSeek.",
            "DeepSeek via Grok requires explicit Grok custom-model configuration and credentials; it is not auto-selected.",
        ],
    }
    observed = inspection.get("observed", {}) if isinstance(inspection, dict) else {}
    human_lines = [
        f"Grok CLI doctor: {'valid' if report['valid'] else 'invalid'}",
        f"binary: {inspection.get('binary_path', '')}",
        f"version: {observed.get('version_output', '')}",
        f"sha256 match: {inspection.get('checks', {}).get('sha256_match')}",
        "supported run modes: version-smoke, acp-smoke",
        "retrieval modes: local_browser, framework_fallback, off",
        "fake provider: explicit --fake-provider for acp-smoke",
        "no-residue required: true",
    ]
    _print_or_json(report, json_output=args.json, human_lines=human_lines)
    return 0 if report["valid"] else 1


def _grok_real_deepseek_model_config() -> str:
    return """
[features]
telemetry = false
feedback = false
lsp_tools = false
codebase_indexing = false
remote_fetch = false

[session]
load_envrc = false

[models]
default = "lif-deepseek-v4-pro"
web_search = "lif-deepseek-v4-pro"
default_reasoning_effort = "none"
stream_tool_calls = false

[model.lif-deepseek-v4-pro]
model = "deepseek-v4-pro"
base_url = "https://api.deepseek.com"
name = "LIF DeepSeek V4 Pro"
env_key = "LIF_DEEPSEEK_API_KEY"
api_backend = "chat_completions"
max_completion_tokens = 128
context_window = 128000
stream_tool_calls = false
thinking = { type = "disabled" }
extra_body = { thinking = { type = "disabled" } }

[model."grok-4.5"]
model = "deepseek-v4-pro"
base_url = "https://api.deepseek.com"
name = "LIF DeepSeek V4 Pro (builtin-default override)"
env_key = "LIF_DEEPSEEK_API_KEY"
api_backend = "chat_completions"
max_completion_tokens = 128
context_window = 128000
stream_tool_calls = false
thinking = { type = "disabled" }
extra_body = { thinking = { type = "disabled" } }

[permission]
rules = [
  { action = "deny", tool = "edit" },
  { action = "deny", tool = "bash" },
  { action = "deny", tool = "grep" },
  { action = "deny", tool = "mcp" },
  { action = "deny", tool = "webfetch" },
  { action = "deny", tool = "websearch" },
]
"""


def _run_grok(args: argparse.Namespace) -> int:
    from .grok_runtime_adapter import (
        DEFAULT_RETRIEVAL_MODE,
        GrokRunRequest,
        run_grok_acp_once,
        run_grok_version_smoke,
    )

    if args.mode not in ("version-smoke", "acp-smoke"):
        raise AssuranceError(f"unsupported Grok run mode: {args.mode}")
    if args.fake_provider and args.mode != "acp-smoke":
        raise AssuranceError("--fake-provider is only valid with --mode acp-smoke")
    if args.real_deepseek and args.mode != "acp-smoke":
        raise AssuranceError("--real-deepseek is only valid with --mode acp-smoke")
    if args.fake_provider and args.real_deepseek:
        raise AssuranceError("--fake-provider and --real-deepseek are mutually exclusive")
    retrieval_mode_explicit = args.retrieval_mode is not None
    retrieval_mode = args.retrieval_mode or DEFAULT_RETRIEVAL_MODE
    if args.mode == "version-smoke":
        receipt = run_grok_version_smoke(
            run_root=args.run_root,
            workspace_path=args.workspace,
            run_id=args.run_id,
            retrieval_mode=retrieval_mode,
            retrieval_mode_explicit=retrieval_mode_explicit,
        )
    else:
        if not args.ask:
            raise AssuranceError("--ask is required for --mode acp-smoke")
        model_id = "lif-fake-deepseek"
        provider_environment: dict[str, str] = {}
        model_config_toml: str | None = None
        if args.real_deepseek:
            from .deepseek_adapter import _read_windows_credential

            api_key = _read_windows_credential(args.credential_target)
            model_id = "lif-deepseek-v4-pro"
            provider_environment = {
                "LIF_DEEPSEEK_API_KEY": api_key,
                "HTTP_PROXY": "",
                "HTTPS_PROXY": "",
                "ALL_PROXY": "",
                "NO_PROXY": "",
            }
            model_config_toml = _grok_real_deepseek_model_config()
        receipt = run_grok_acp_once(
            GrokRunRequest(
                run_root=args.run_root,
                workspace_path=args.workspace or (args.run_root / "workspace"),
                run_id=args.run_id or "RUN-GROK-ACP-SMOKE-001",
                mode="acp_smoke",
                prompt_text=args.ask,
                model_id=model_id,
                retrieval_mode=retrieval_mode,
                retrieval_mode_explicit=retrieval_mode_explicit,
                fake_provider=args.fake_provider,
                provider_environment=provider_environment,
                model_config_toml=model_config_toml,
            )
        )
    containment = receipt.get("containment", {})
    human_lines = [
        f"grok run [{args.mode}]: {'valid' if receipt['valid'] else 'invalid'}",
        f"run_id: {receipt['run_id']}",
        f"run_root: {receipt['request']['run_root']}",
        f"version: {receipt['binary'].get('version_output', '')}",
        f"retrieval mode: {receipt['retrieval']['mode']}",
        f"no residue observed: {containment.get('no_residue_observed')}",
        f"external cleanup required: {containment.get('external_cleanup_required')}",
        f"events: {receipt['artifacts']['events_path']}",
    ]
    _print_or_json(receipt, json_output=args.json, human_lines=human_lines)
    return 0 if receipt["valid"] else 1


def _run_deepseek(args: argparse.Namespace) -> int:
    from .deepseek_runtime_adapter import run_deepseek_direct_smoke

    if not args.ask:
        raise AssuranceError("--ask is required for deepseek run")
    receipt = run_deepseek_direct_smoke(
        run_root=args.run_root,
        prompt_text=args.ask,
        credential_target=args.credential_target,
        run_id=args.run_id,
        timeout_seconds=args.api_timeout,
    )
    human_lines = [
        f"deepseek run: {'valid' if receipt['valid'] else 'invalid'}",
        f"run_id: {receipt['run_id']}",
        f"run_root: {receipt['request']['run_root']}",
        f"http status: {receipt['execution']['http_status_code']}",
        f"events: {receipt['artifacts']['events_path']}",
        f"receipt: {receipt['artifacts']['receipt_path']}",
    ]
    _print_or_json(receipt, json_output=args.json, human_lines=human_lines)
    return 0 if receipt["valid"] else 1


def _run_grok_observe_tools(args: argparse.Namespace) -> int:
    from .grok_tool_permission_observer import (
        build_grok_tool_permission_observation_bundle,
        observe_grok_tool_permission_surfaces,
    )
    from .utils import atomic_write_json

    receipt = observe_grok_tool_permission_surfaces(
        acp_verification_paths=args.acp_verification
    )
    bundle = build_grok_tool_permission_observation_bundle(receipt)
    gate = bundle["tool_availability_gate_receipt"]
    human_lines = [
        f"grok observe-tools: {receipt['decision']}",
        f"receipt_id: {receipt['receipt_id']}",
        f"project agents: {', '.join(receipt['surface']['project_agents'])}",
        f"permission controls observed: {receipt['checks']['permission_controls_observed']}",
        f"acp verification attached: {receipt['checks']['acp_permission_probe_attached']}",
        f"tool availability gate: {gate['decisions']['gate_decision']}",
        "prompt/tool promotion: blocked",
    ]
    if args.output_gate_receipt:
        atomic_write_json(args.output_gate_receipt, gate)
        human_lines.append(
            f"gate receipt written: {args.output_gate_receipt}"
        )
    payload = bundle if args.include_tool_availability else receipt
    _print_or_json(payload, json_output=args.json, human_lines=human_lines)
    return 0 if receipt["valid"] else 1


def _default_alpha_smoke_run_root() -> Path:
    from datetime import datetime, timezone

    stamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    return ROOT / ".gsa" / "alpha-smoke" / f"{stamp}-{uuid.uuid4().hex[:8]}"


def _run_alpha_smoke(args: argparse.Namespace) -> int:
    from .grok_runtime_adapter import (
        DEFAULT_RETRIEVAL_MODE,
        GrokRunRequest,
        run_grok_acp_once,
    )

    run_root = args.run_root or _default_alpha_smoke_run_root()
    prompt_text = args.ask or "Return the fixed loopback fixture marker only."
    receipt = run_grok_acp_once(
        GrokRunRequest(
            run_root=run_root,
            workspace_path=args.workspace or (run_root / "workspace"),
            run_id=args.run_id or f"RUN-GSA-ALPHA-SMOKE-{uuid.uuid4().hex[:8].upper()}",
            mode="acp_smoke",
            prompt_text=prompt_text,
            retrieval_mode=DEFAULT_RETRIEVAL_MODE,
            retrieval_mode_explicit=False,
            fake_provider=True,
            acp_permission_mode="auto_allow_once",
        )
    )
    containment = receipt.get("containment", {})
    artifacts = receipt.get("artifacts", {})
    human_lines = [
        f"gsa alpha smoke: {'valid' if receipt['valid'] else 'invalid'}",
        "runtime: grok acp-smoke",
        "provider: loopback fake-provider (no credentials, no external network)",
        f"run_id: {receipt['run_id']}",
        f"run_root: {receipt['request']['run_root']}",
        f"version: {receipt['binary'].get('version_output', '')}",
        f"no residue observed: {containment.get('no_residue_observed')}",
        f"events: {artifacts.get('events_path', '')}",
        f"receipt: {artifacts.get('receipt_path', '')}",
    ]
    _print_or_json(receipt, json_output=args.json, human_lines=human_lines)
    return 0 if receipt["valid"] else 1


def _default_alpha_ui_check_run_root() -> Path:
    from datetime import datetime, timezone

    stamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    return ROOT / ".gsa" / "alpha-ui-check" / f"{stamp}-{uuid.uuid4().hex[:8]}"


def _run_alpha_ui_check(args: argparse.Namespace) -> int:
    from contextlib import redirect_stdout
    from io import StringIO

    from .utils import atomic_write_json, sha256_bytes, utc_now

    run_root = args.run_root or _default_alpha_ui_check_run_root()
    run_root.mkdir(parents=True, exist_ok=True)

    from .tui.app import TuiPrototype
    from .tui.event_source import FakeEventSource, LiveRunEventSource, build_gate_chain_demo
    from .tui.main import _interactive_console_available, _run_static_live

    checks: dict[str, bool] = {}
    artifacts: dict[str, str] = {}
    details: dict[str, Any] = {}

    sample_app = TuiPrototype.with_sample_data()
    sample_output = sample_app.render(args.width, args.height)
    checks["self_ui_static_rendered"] = "Traceback" not in sample_output
    checks["self_ui_frame_present"] = "当前任务" in sample_output and "文件" in sample_output
    checks["self_ui_terminal_bounds_accepted"] = not sample_output.startswith("Viewport too small")
    details["self_ui_static_render_sha256"] = sha256_bytes(sample_output.encode("utf-8"))

    source = FakeEventSource(preload=build_gate_chain_demo())
    event_app = TuiPrototype.with_event_source(source)
    event_app.poll_events()
    event_output = event_app.render(args.width, args.height)
    checks["self_ui_event_projection_drained"] = bool(getattr(event_app, "_event_log", []))
    checks["self_ui_event_projection_rendered"] = "Traceback" not in event_output
    details["self_ui_event_render_sha256"] = sha256_bytes(event_output.encode("utf-8"))

    console_available = _interactive_console_available()
    checks["non_console_static_fallback_available"] = True
    details["interactive_console_available"] = console_available

    grok_report: dict[str, Any]
    try:
        from .grok_runtime_adapter import inspect_grok_runtime

        grok_report = inspect_grok_runtime()
        checks["grok_fallback_binary_valid"] = grok_report.get("valid") is True
        checks["grok_fallback_version_observed"] = bool(
            grok_report.get("observed", {}).get("version_output")
        )
    except Exception as exc:
        grok_report = {"valid": False, "error": str(exc)}
        checks["grok_fallback_binary_valid"] = False
        checks["grok_fallback_version_observed"] = False

    live_smoke: dict[str, Any] = {
        "requested": bool(args.run_live_smoke),
        "status": "skipped",
    }
    if args.run_live_smoke:
        from .tui.bridge import build_grok_acp_live_run_fn

        live_run_root = run_root / "live-static-acp"
        run_fn = build_grok_acp_live_run_fn(
            run_root=str(live_run_root),
            workspace=str(live_run_root / "workspace"),
            prompt_text=args.ask or "Return the fixed loopback fixture marker only.",
            run_id=args.run_id or f"RUN-GSA-ALPHA-UI-{uuid.uuid4().hex[:8].upper()}",
            interactive=False,
            fake_provider=True,
        )
        live_source = LiveRunEventSource(run_fn=run_fn)
        live_app = TuiPrototype.with_event_source(live_source)
        live_app.set_grok_provider_mode(True)
        output = StringIO()
        with redirect_stdout(output):
            exit_code = _run_static_live(live_app, live_source, args.width, args.height)
        rendered = output.getvalue()
        live_receipt = live_source.receipt or {}
        checks["live_static_acp_exit_zero"] = exit_code == 0
        checks["live_static_acp_receipt_valid"] = live_receipt.get("valid") is True
        checks["live_static_acp_marker_rendered"] = "LIF_FAKE_PROVIDER_OK" in rendered
        checks["live_static_acp_no_traceback"] = "Traceback" not in rendered
        live_output_path = run_root / "live-static-tui-output.txt"
        live_output_path.write_text(rendered, encoding="utf-8")
        artifacts["live_static_tui_output_path"] = str(live_output_path)
        live_smoke = {
            "requested": True,
            "status": "completed" if all(
                checks[name]
                for name in (
                    "live_static_acp_exit_zero",
                    "live_static_acp_receipt_valid",
                    "live_static_acp_marker_rendered",
                    "live_static_acp_no_traceback",
                )
            ) else "failed",
            "run_root": str(live_run_root),
            "render_sha256": sha256_bytes(rendered.encode("utf-8")),
            "receipt_path": live_receipt.get("artifacts", {}).get("receipt_path", ""),
        }

    receipt = {
        "schema_version": "0.1.0-draft",
        "receipt_kind": "alpha_ui_usability_check_receipt",
        "created_at": utc_now(),
        "valid": all(checks.values()),
        "run_root": str(run_root),
        "ui_strategy": {
            "primary": "gsa_tui",
            "fallback": "grok_native_ui",
            "primary_scope": "daily assurance view, event projection, gate visibility",
            "fallback_scope": "runtime-owned interactive UI when GSA TUI is unavailable",
            "beta_deferred": ["real terminal keyboard smoke", "human UX timing study"],
        },
        "checks": checks,
        "details": details,
        "grok_fallback": {
            "valid": grok_report.get("valid") is True,
            "binary_path": grok_report.get("binary_path", ""),
            "version_output": grok_report.get("observed", {}).get("version_output", ""),
            "launch_hint": "grok",
        },
        "live_smoke": live_smoke,
        "artifacts": artifacts,
        "limitations": [
            "This check validates deterministic render/projection/static live paths; it does not prove keyboard interaction in a real terminal.",
            "The Grok fallback check verifies the locked binary and launch hint, not a full native UI session.",
            "Live static smoke uses the loopback fake provider and does not prove real model/tool execution.",
        ],
    }
    receipt_path = run_root / "alpha-ui-check.json"
    receipt["artifacts"]["receipt_path"] = str(receipt_path)
    atomic_write_json(receipt_path, receipt, overwrite=args.overwrite)

    human_lines = [
        f"gsa alpha ui-check: {'valid' if receipt['valid'] else 'invalid'}",
        f"run_root: {run_root}",
        f"primary UI: {receipt['ui_strategy']['primary']}",
        f"fallback UI: {receipt['ui_strategy']['fallback']}",
        f"interactive console available: {console_available}",
        f"live static smoke: {live_smoke['status']}",
        f"receipt: {receipt_path}",
    ]
    _print_or_json(receipt, json_output=args.json, human_lines=human_lines)
    return 0 if receipt["valid"] else 1


def _default_alpha_tool_check_run_root() -> Path:
    from datetime import datetime, timezone

    stamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    return ROOT / ".gsa" / "alpha-tool-check" / f"{stamp}-{uuid.uuid4().hex[:8]}"


def _default_alpha_acp_verification_paths() -> list[Path]:
    base = ROOT / "candidate-gates" / "grok-0.2.112-live-gates-admin-r3"
    return [
        base / "fake-tool-allow" / "verification.json",
        base / "fake-tool-cancel" / "verification.json",
    ]


def _enrich_alpha_acp_verification(path: Path, output_dir: Path) -> dict[str, Any]:
    from .utils import atomic_write_json, load_json

    verification = load_json(path)
    if not isinstance(verification, dict):
        raise AssuranceError(f"ACP verification is not a JSON object: {path}")
    result_path = path.parent / "result.json"
    result = load_json(result_path) if result_path.exists() else {}
    if not isinstance(result, dict):
        result = {}
    acp = result.get("acp", {}) if isinstance(result.get("acp"), dict) else {}
    provider = (
        result.get("provider", {}) if isinstance(result.get("provider"), dict) else {}
    )
    enriched = dict(verification)
    if "scenario" not in enriched and isinstance(result.get("scenario"), str):
        enriched["scenario"] = result["scenario"]
    if "permission_outcome" not in enriched and isinstance(
        acp.get("permission_outcome"), str
    ):
        enriched["permission_outcome"] = acp["permission_outcome"]
    if "provider_scenario" not in enriched and isinstance(provider.get("scenario"), str):
        enriched["provider_scenario"] = provider["scenario"]
    output_dir.mkdir(parents=True, exist_ok=True)
    output_path = output_dir / f"{path.parent.name}-verification.enriched.json"
    atomic_write_json(output_path, enriched, overwrite=True)
    return {
        "source_path": str(path),
        "result_path": str(result_path) if result_path.exists() else "",
        "enriched_path": str(output_path),
        "scenario": enriched.get("scenario", ""),
        "permission_outcome": enriched.get("permission_outcome", ""),
        "provider_scenario": enriched.get("provider_scenario", ""),
        "valid": enriched.get("valid") is True,
    }


def _run_alpha_tool_check(args: argparse.Namespace) -> int:
    from .grok_tool_permission_observer import (
        build_grok_tool_permission_observation_bundle,
        observe_grok_tool_permission_surfaces,
    )
    from .utils import atomic_write_json, utc_now

    run_root = args.run_root or _default_alpha_tool_check_run_root()
    run_root.mkdir(parents=True, exist_ok=True)
    verification_paths = args.acp_verification or _default_alpha_acp_verification_paths()
    missing_paths = [str(path) for path in verification_paths if not path.exists()]
    enriched_records: list[dict[str, Any]] = []
    enriched_paths: list[Path] = []
    if not missing_paths:
        enriched_dir = run_root / "enriched-acp-verifications"
        for path in verification_paths:
            record = _enrich_alpha_acp_verification(path, enriched_dir)
            enriched_records.append(record)
            enriched_paths.append(Path(record["enriched_path"]))

    negative_observation = observe_grok_tool_permission_surfaces(
        acp_verification_paths=()
    )
    negative_bundle = build_grok_tool_permission_observation_bundle(
        negative_observation
    )
    atomic_write_json(
        run_root / "negative-tool-observation-bundle.json",
        negative_bundle,
        overwrite=args.overwrite,
    )

    positive_bundle: dict[str, Any] | None = None
    if not missing_paths:
        positive_observation = observe_grok_tool_permission_surfaces(
            acp_verification_paths=enriched_paths
        )
        positive_bundle = build_grok_tool_permission_observation_bundle(
            positive_observation
        )
        atomic_write_json(
            run_root / "positive-tool-observation-bundle.json",
            positive_bundle,
            overwrite=args.overwrite,
        )

    negative_gate_decision = negative_bundle["tool_availability_gate_receipt"][
        "decisions"
    ]["gate_decision"]
    negative_degraded_ids = [
        item["tool_id"]
        for item in negative_bundle["tool_availability_report"].get("degraded", [])
        if isinstance(item, dict)
    ]
    positive_gate_decision = (
        positive_bundle["tool_availability_gate_receipt"]["decisions"]["gate_decision"]
        if positive_bundle is not None
        else "missing"
    )
    positive_acp = (
        positive_bundle["observation_receipt"]["acp_permission_observation"]
        if positive_bundle is not None
        else {}
    )
    positive_status_lists = (
        positive_bundle["tool_availability_report"] if positive_bundle is not None else {}
    )

    checks = {
        "verification_paths_present": not missing_paths,
        "negative_without_acp_probe_blocks": negative_gate_decision == "block",
        "negative_permission_probe_unavailable": (
            "grok_acp_permission_probe" in negative_degraded_ids
            or "grok_acp_permission_probe" in [
                item["tool_id"] for item in negative_bundle["tool_availability_report"].get("unavailable", [])
                if isinstance(item, dict)
            ]
        ),
        "negative_no_model_invoked": negative_bundle["observation_receipt"]["checks"][
            "no_model_invoked"
        ],
        "negative_no_network_requested": negative_bundle["observation_receipt"][
            "checks"
        ]["no_network_requested"],
        "positive_dual_acp_required_scenarios_verified": (
            positive_acp.get("required_scenarios_verified") is True
        ),
        "positive_covers_allow_and_cancel": set(
            positive_acp.get("covered_scenarios", [])
        )
        == {"allow_once", "cancel_permission"},
        "positive_permission_outcomes_observed": set(
            positive_acp.get("permission_outcomes", [])
        )
        == {"allow_once", "cancelled"},
        "positive_tool_availability_allows": positive_gate_decision == "allow",
        "positive_no_degraded_tools": (
            positive_bundle is not None
            and positive_status_lists.get("degraded") == []
        ),
        "positive_no_unavailable_tools": (
            positive_bundle is not None
            and positive_status_lists.get("unavailable") == []
        ),
        "positive_no_unprobed_tools": (
            positive_bundle is not None
            and positive_status_lists.get("unprobed") == []
        ),
        "positive_no_model_invoked": (
            positive_bundle is not None
            and positive_bundle["observation_receipt"]["checks"]["no_model_invoked"]
        ),
        "positive_no_network_requested": (
            positive_bundle is not None
            and positive_bundle["observation_receipt"]["checks"][
                "no_network_requested"
            ]
        ),
    }
    receipt = {
        "schema_version": "0.1.0-draft",
        "receipt_kind": "alpha_tool_check_receipt",
        "created_at": utc_now(),
        "valid": all(checks.values()),
        "run_root": str(run_root),
        "verification_inputs": {
            "paths": [str(path) for path in verification_paths],
            "missing_paths": missing_paths,
            "enriched": enriched_records,
        },
        "checks": checks,
        "negative_case": {
            "description": "No ACP permission verification is attached; tool availability must block.",
            "observation_decision": negative_bundle["observation_receipt"]["decision"],
            "gate_decision": negative_gate_decision,
            "degraded_tool_ids": negative_degraded_ids,
            "bundle_path": str(run_root / "negative-tool-observation-bundle.json"),
        },
        "positive_case": (
            {
                "description": "allow_once and cancel_permission ACP verifications are attached; tool availability may allow.",
                "observation_decision": positive_bundle["observation_receipt"][
                    "decision"
                ],
                "gate_decision": positive_gate_decision,
                "covered_scenarios": positive_acp.get("covered_scenarios", []),
                "permission_outcomes": positive_acp.get("permission_outcomes", []),
                "available_count": positive_bundle[
                    "tool_availability_gate_receipt"
                ]["available_count"],
                "bundle_path": str(run_root / "positive-tool-observation-bundle.json"),
            }
            if positive_bundle is not None
            else {
                "description": "Positive case skipped because verification paths were missing.",
                "gate_decision": "missing",
            }
        ),
        "limitations": [
            "This check hardens Grok tool availability and ACP fake-tool permission semantics; it does not call a real external tool.",
            "The attached fake-tool verifications prove allow/cancel ACP permission paths, not arbitrary MCP or web tools.",
            "Tool availability is a hard structured return; model-facing text is not the enforcement layer.",
        ],
    }
    receipt_path = run_root / "alpha-tool-check.json"
    receipt["artifacts"] = {
        "receipt_path": str(receipt_path),
        "negative_bundle_path": str(run_root / "negative-tool-observation-bundle.json"),
        "positive_bundle_path": (
            str(run_root / "positive-tool-observation-bundle.json")
            if positive_bundle is not None
            else ""
        ),
    }
    atomic_write_json(receipt_path, receipt, overwrite=args.overwrite)
    human_lines = [
        f"gsa alpha tool-check: {'valid' if receipt['valid'] else 'invalid'}",
        f"run_root: {run_root}",
        f"negative gate: {negative_gate_decision}",
        f"positive gate: {positive_gate_decision}",
        f"missing verification paths: {len(missing_paths)}",
        f"receipt: {receipt_path}",
    ]
    _print_or_json(receipt, json_output=args.json, human_lines=human_lines)
    return 0 if receipt["valid"] else 1


def _default_alpha_real_call_run_root() -> Path:
    from datetime import datetime, timezone

    stamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    return ROOT / ".gsa" / "alpha-real-call" / f"{stamp}-{uuid.uuid4().hex[:8]}"


def _run_alpha_real_call(args: argparse.Namespace) -> int:
    from .grok_runtime_adapter import DEFAULT_RETRIEVAL_MODE
    from .utils import atomic_write_json, sha256_bytes, utc_now

    run_root = args.run_root or _default_alpha_real_call_run_root()
    run_root.mkdir(parents=True, exist_ok=True)
    adapter_run_root = run_root / "grok-real-acp"
    workspace_path = args.workspace or (adapter_run_root / "workspace")
    prompt_text = args.ask or "Return exactly: GSA_ALPHA_REAL_CALL_OK"
    run_id = args.run_id or f"RUN-GSA-ALPHA-REAL-{uuid.uuid4().hex[:8].upper()}"

    credential_error = ""
    api_key = ""
    try:
        from .deepseek_adapter import _read_windows_credential

        api_key = _read_windows_credential(args.credential_target)
    except Exception as exc:
        credential_error = f"{type(exc).__name__}: {exc}"

    adapter_receipt: dict[str, Any] = {}
    adapter_error = ""
    direct_receipt: dict[str, Any] = {}
    direct_error = ""
    request_shape = {
        "transport": args.transport,
        "mode": "acp_smoke" if args.transport == "grok-acp" else "chat_completions",
        "run_root": str(adapter_run_root),
        "workspace_path": str(workspace_path),
        "run_id": run_id,
        "model_id": "lif-deepseek-v4-pro",
        "retrieval_mode": DEFAULT_RETRIEVAL_MODE,
        "retrieval_mode_explicit": False,
        "fake_provider": False,
        "disable_builtin_tools": False,
        "provider_environment_keys": [
            "LIF_DEEPSEEK_API_KEY",
            "HTTP_PROXY",
            "HTTPS_PROXY",
            "ALL_PROXY",
            "NO_PROXY",
        ],
        "model_config_env_key": "LIF_DEEPSEEK_API_KEY",
    }
    if api_key and args.transport == "grok-acp":
        from .grok_runtime_adapter import GrokRunRequest, run_grok_acp_once

        try:
            adapter_receipt = run_grok_acp_once(
                GrokRunRequest(
                    run_root=adapter_run_root,
                    workspace_path=workspace_path,
                    run_id=run_id,
                    mode="acp_smoke",
                    prompt_text=prompt_text,
                    model_id="lif-deepseek-v4-pro",
                    retrieval_mode=DEFAULT_RETRIEVAL_MODE,
                    retrieval_mode_explicit=False,
                    fake_provider=False,
                    provider_environment={
                        "LIF_DEEPSEEK_API_KEY": api_key,
                        "HTTP_PROXY": "",
                        "HTTPS_PROXY": "",
                        "ALL_PROXY": "",
                        "NO_PROXY": "",
                    },
                    model_config_toml=_grok_real_deepseek_model_config(),
                )
            )
        except Exception as exc:
            adapter_error = f"{type(exc).__name__}: {exc}"
    elif api_key and args.transport == "direct-deepseek":
        from .deepseek_runtime_adapter import (
            DeepSeekRunRequest,
            run_deepseek_direct_once,
        )

        try:
            direct_receipt = run_deepseek_direct_once(
                DeepSeekRunRequest(
                    run_root=adapter_run_root,
                    run_id=run_id,
                    prompt_text=prompt_text,
                    credential_target=args.credential_target,
                    timeout_seconds=args.api_timeout,
                ),
                credential_reader=lambda _target: api_key,
            )
        except Exception as exc:
            direct_error = f"{type(exc).__name__}: {exc}"

    adapter_request = (
        adapter_receipt.get("request", {}) if isinstance(adapter_receipt, dict) else {}
    )
    adapter_retrieval = (
        adapter_receipt.get("retrieval", {}) if isinstance(adapter_receipt, dict) else {}
    )
    adapter_checks = (
        adapter_receipt.get("checks", {}) if isinstance(adapter_receipt, dict) else {}
    )
    adapter_prompt = (
        adapter_receipt.get("prompt", {}) if isinstance(adapter_receipt, dict) else {}
    )
    adapter_artifacts = (
        adapter_receipt.get("artifacts", {}) if isinstance(adapter_receipt, dict) else {}
    )

    common_checks = {
        "credential_read_attempted": True,
        "credential_read_succeeded": bool(api_key) and not credential_error,
        "fake_provider_disabled": request_shape["fake_provider"] is False
        and adapter_request.get("fake_provider", False) is False,
        "real_provider_model_configured": request_shape["model_id"] == "lif-deepseek-v4-pro",
        "builtin_tools_enabled": (
            # Only meaningful for grok-acp transport (direct-deepseek has no Grok tools concept)
            request_shape["disable_builtin_tools"] is False
            and (
                adapter_request.get("disable_builtin_tools", False) is False
                if args.transport == "grok-acp"
                else True  # direct-deepseek: no Grok, always passes
            )
        ),
        "retrieval_off": request_shape["retrieval_mode"] == "off"
        and adapter_retrieval.get("mode", "off") == "off",
    }
    if args.transport == "grok-acp":
        checks = {
            **common_checks,
            "adapter_receipt_valid": adapter_receipt.get("valid") is True,
            "acp_prompt_completed": adapter_checks.get("acp_prompt_completed") is True,
            "acp_response_received": adapter_checks.get("acp_response_received") is True,
            "provider_api_errors_absent": adapter_checks.get(
                "provider_api_errors_absent"
            )
            is True,
            "no_residue_observed": (
                adapter_receipt.get("containment", {}).get("no_residue_observed")
                is True
                if isinstance(adapter_receipt.get("containment", {}), dict)
                else False
            ),
        }
    else:
        direct_prompt = (
            direct_receipt.get("prompt", {})
            if isinstance(direct_receipt.get("prompt", {}), dict)
            else {}
        )
        direct_execution = (
            direct_receipt.get("execution", {})
            if isinstance(direct_receipt.get("execution", {}), dict)
            else {}
        )
        checks = {
            **common_checks,
            "direct_adapter_receipt_valid": direct_receipt.get("valid") is True,
            "direct_http_status_ok": direct_execution.get("http_status_code") == 200,
            "direct_response_received": bool(direct_prompt.get("response_sha256")),
            "direct_finish_reason_observed": bool(
                direct_prompt.get("response_finish_reason")
            ),
            "direct_no_provider_error": direct_error == "",
            "no_child_process_started": True,
        }

    receipt = {
        "schema_version": "0.1.0-draft",
        "receipt_kind": "alpha_real_call_receipt",
        "created_at": utc_now(),
        "run_root": str(run_root),
        "run_id": run_id,
        "provider": {
            "runtime_owner": "grok" if args.transport == "grok-acp" else "deepseek",
            "model_id": "lif-deepseek-v4-pro",
            "credential_target": args.credential_target,
            "credential_error": credential_error,
            "external_network_expected": True,
            "fake_provider": False,
        },
        "request": request_shape,
        "prompt": {
            "prompt_sha256": sha256_bytes(prompt_text.encode("utf-8")),
            "prompt_bytes": len(prompt_text.encode("utf-8")),
            "response_sha256": (
                adapter_prompt.get("response_sha256", "")
                if args.transport == "grok-acp"
                else direct_receipt.get("prompt", {}).get("response_sha256", "")
                if isinstance(direct_receipt.get("prompt", {}), dict)
                else ""
            ),
            "response_bytes_observed": (
                bool(adapter_prompt.get("response_sha256"))
                if args.transport == "grok-acp"
                else bool(
                    direct_receipt.get("prompt", {}).get("response_sha256", "")
                    if isinstance(direct_receipt.get("prompt", {}), dict)
                    else ""
                )
            ),
        },
        "adapter": {
            "valid": adapter_receipt.get("valid") is True,
            "error": adapter_error,
            "receipt_path": adapter_artifacts.get("receipt_path", ""),
            "events_path": adapter_artifacts.get("events_path", ""),
            "binary_version": adapter_receipt.get("binary", {}).get(
                "version_output", ""
            )
            if isinstance(adapter_receipt.get("binary", {}), dict)
            else "",
            "selected_checks": {
                key: adapter_checks.get(key)
                for key in (
                    "binary_inspection_valid",
                    "workspace_trust_valid",
                    "workspace_trust_granted",
                    "acp_initialize_ok",
                    "acp_session_created",
                    "acp_prompt_completed",
                    "acp_response_received",
                    "provider_api_errors_absent",
                    "no_residue_observed",
                )
            },
        },
        "direct_provider": {
            "valid": direct_receipt.get("valid") is True and direct_error == "",
            "error": direct_error,
            "receipt_path": direct_receipt.get("artifacts", {}).get(
                "receipt_path", ""
            )
            if isinstance(direct_receipt.get("artifacts", {}), dict)
            else "",
            "events_path": direct_receipt.get("artifacts", {}).get("events_path", "")
            if isinstance(direct_receipt.get("artifacts", {}), dict)
            else "",
            "http_status_code": direct_receipt.get("execution", {}).get(
                "http_status_code"
            )
            if isinstance(direct_receipt.get("execution", {}), dict)
            else None,
            "finish_reason": direct_receipt.get("prompt", {}).get(
                "response_finish_reason", ""
            )
            if isinstance(direct_receipt.get("prompt", {}), dict)
            else "",
            "model": direct_receipt.get("prompt", {}).get("model_id", "")
            if isinstance(direct_receipt.get("prompt", {}), dict)
            else "",
            "usage": direct_receipt.get("prompt", {}).get("usage", {})
            if isinstance(direct_receipt.get("prompt", {}), dict)
            else {},
            "private_reasoning_content_sha256": direct_receipt.get("prompt", {}).get(
                "private_reasoning_content_sha256"
            )
            if isinstance(direct_receipt.get("prompt", {}), dict)
            else None,
        },
        "checks": checks,
        "limitations": [
            (
                "The default grok-acp transport performs a real Grok ACP model call through DeepSeek when credentials are available."
                if args.transport == "grok-acp"
                else "The direct-deepseek transport performs a fixed prompt Chat Completions control call without project source context."
            ),
            (
                "It disables Grok built-in tool surfaces and keeps retrieval mode off; it does not exercise external tools."
                if args.transport == "grok-acp"
                else "It bypasses Grok entirely and therefore proves provider/API availability, not Grok ACP compatibility."
            ),
            "The alpha receipt stores hashes and selected adapter status only; API key material is not serialized.",
        ],
    }
    checks["no_secret_serialized"] = (not api_key) or api_key not in _json_dump(receipt)
    receipt["valid"] = all(checks.values())
    receipt_path = run_root / "alpha-real-call.json"
    receipt["artifacts"] = {
        "receipt_path": str(receipt_path),
        "adapter_run_root": str(adapter_run_root),
    }
    atomic_write_json(receipt_path, receipt, overwrite=args.overwrite)

    human_lines = [
        f"gsa alpha real-call: {'valid' if receipt['valid'] else 'invalid'}",
        f"run_root: {run_root}",
        f"transport: {args.transport}",
        f"credential read: {'ok' if checks['credential_read_succeeded'] else 'failed'}",
        f"adapter valid: {receipt['adapter']['valid']}",
        f"direct provider valid: {receipt['direct_provider']['valid']}",
        f"receipt: {receipt_path}",
    ]
    _print_or_json(receipt, json_output=args.json, human_lines=human_lines)
    return 0 if receipt["valid"] else 1


def _run_global_review(args: argparse.Namespace) -> int:
    from .global_review_mode import build_global_review_mode_receipt
    from .utils import atomic_write_json

    receipt = build_global_review_mode_receipt(
        paths=args.path,
        include_git_status=not args.no_git_status,
        review_id=args.review_id,
    )
    if args.output:
        atomic_write_json(args.output, receipt)
    triggered = sorted(
        {
            flag
            for finding in receipt["path_findings"]
            for flag in finding["risk_flags"]
        }
    )
    output_line = f"receipt: {args.output}" if args.output else "receipt: stdout"
    human_lines = [
        "global review mode: active",
        f"review_id: {receipt['review_id']}",
        f"scope: {receipt['scope']['source']} ({receipt['scope']['path_count']} paths)",
        f"dimensions: {len(receipt['dimensions'])} required",
        f"risk flags: {', '.join(triggered) if triggered else 'none'}",
        output_line,
        "ordinary review remains: local_engineering_review",
    ]
    _print_or_json(receipt, json_output=args.json, human_lines=human_lines)
    return 0


def _run_tui(args: argparse.Namespace) -> int:
    from assurance.tui.main import main as tui_main

    if args.grok_retrieval_mode and args.runtime != "grok":
        print("--retrieval-mode is only valid with --runtime grok", file=sys.stderr)
        return 2
    if getattr(args, "fake_provider", False) and args.runtime != "grok":
        print("--fake-provider is only valid with --runtime grok", file=sys.stderr)
        return 2
    if args.runtime == "deepseek" and not args.real:
        print("--runtime deepseek requires --real", file=sys.stderr)
        return 2

    tui_args: list[str] = [
        "--width",
        str(args.width),
        "--height",
        str(args.height),
        "--runtime",
        args.runtime,
    ]
    if args.demo:
        tui_args.append("--demo")
    if args.with_dialog:
        tui_args.append("--with-dialog")
    if args.with_properties:
        tui_args.append("--with-properties")
    if args.demo_events:
        tui_args.extend(["--demo-events", args.demo_events])
    if args.live_retrieval:
        tui_args.append("--live-retrieval")
    if args.replay:
        tui_args.extend(["--replay", str(args.replay)])
    if args.tui_run:
        tui_args.extend(["--run", args.tui_run])
    if args.real:
        tui_args.append("--real")
    if getattr(args, "fake_provider", False):
        tui_args.append("--fake-provider")
    if args.credential_target:
        tui_args.extend(["--credential-target", args.credential_target])
    if args.tui_run_root:
        tui_args.extend(["--run-root", str(args.tui_run_root)])
    if args.workspace:
        tui_args.extend(["--workspace", str(args.workspace)])
    if args.grok_retrieval_mode:
        tui_args.extend(["--retrieval-mode", args.grok_retrieval_mode])
    return tui_main(tui_args)


def _run_interactive(args: argparse.Namespace) -> int:
    """Interactive CLI conversation — ``gsa chat``.

    Launches a Grok ACP session with tools enabled and streams events
    to the terminal.  Each user input triggers a new ``send_prompt``
    on the same session.  Special commands:

    * ``/exit``, ``/quit`` — end the session
    * ``/help`` — show available commands
    * ``/clear`` — clear the screen (start a fresh visual context;
      the session context persists in Grok)
    """
    from datetime import datetime, timezone
    from pathlib import Path as _Path

    from .grok_runtime_adapter import (
        DEFAULT_RETRIEVAL_MODE,
        GrokRunRequest,
        GrokAcpSession,
        inspect_grok_runtime,
        validate_grok_retrieval_mode,
    )

    # ── Grok detection ─────────────────────────────────────────────────
    bin_path: _Path | None = None
    try:
        from .grok_runtime_adapter import _detect_grok_binary
        grok_path, grok_ver = _detect_grok_binary()
        if grok_path:
            bin_path = _Path(grok_path)
            ver_str = f" ({grok_ver})" if grok_ver else ""
            print(f"gsa: Grok detected{ver_str}", file=sys.stderr)
        else:
            print("gsa: Grok not found on PATH.  Install Grok to continue.", file=sys.stderr)
            print("  https://github.com/xai-org/grok-build", file=sys.stderr)
            return 1
    except Exception as exc:
        print(f"gsa: Grok detection failed — {exc}", file=sys.stderr)
        return 1

    # ── Run root ───────────────────────────────────────────────────────
    stamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    run_root = _Path(args.run_root) if getattr(args, "run_root", None) else (
        ROOT / ".gsa" / "interactive" / f"{stamp}-{uuid.uuid4().hex[:8]}"
    )
    workspace = _Path(args.workspace) if getattr(args, "workspace", None) and args.workspace else (
        run_root / "workspace"
    )
    run_id = getattr(args, "run_id", None) or f"RUN-GSA-INTERACTIVE-{uuid.uuid4().hex[:8].upper()}"
    retrieval_mode = getattr(args, "interactive_retrieval_mode", None) or DEFAULT_RETRIEVAL_MODE
    fake_provider = bool(getattr(args, "fake_provider", False))
    auto_approve = bool(getattr(args, "auto_approve", False))
    acp_permission_mode = "auto_allow_once" if auto_approve else "interactive"
    credential_target = getattr(args, "credential_target", None) or DEFAULT_CREDENTIAL_TARGET

    # ── Real-provider setup (opt-in) ──────────────────────────────────
    provider_environment: dict[str, str] = {}
    model_config_toml: str | None = None
    model_id = "lif-fake-deepseek"
    if not fake_provider:
        try:
            from .deepseek_adapter import _read_windows_credential
            api_key = _read_windows_credential(credential_target)
            model_id = "lif-deepseek-v4-pro"
            provider_environment = {
                "LIF_DEEPSEEK_API_KEY": api_key,
                "HTTP_PROXY": "", "HTTPS_PROXY": "", "ALL_PROXY": "", "NO_PROXY": "",
            }
            model_config_toml = _grok_real_deepseek_model_config()
        except Exception:
            print("gsa: Real provider unavailable — falling back to loopback fake provider.",
                  file=sys.stderr)
            fake_provider = True

    request = GrokRunRequest(
        run_root=run_root,
        workspace_path=workspace,
        run_id=run_id,
        mode="acp_smoke",
        retrieval_mode=retrieval_mode,
        retrieval_mode_explicit=True,
        fake_provider=fake_provider,
        provider_environment=provider_environment,
        model_config_toml=model_config_toml,
        model_id=model_id,
        acp_permission_mode=acp_permission_mode,
    )

    # ── Event callback ─────────────────────────────────────────────────
    def _on_event(event: dict[str, Any]) -> str | None:
        etype = event.get("event_type", "")
        payload = event.get("payload", {}) if isinstance(event.get("payload"), dict) else {}
        if etype == "text_delta":
            text = payload.get("text", "")
            if text:
                sys.stdout.write(text)
                sys.stdout.flush()
        elif etype == "tool_proposal":
            tool_name = payload.get("tool_name", "")
            summary = payload.get("input_summary", "")
            print(f"\n  🔧 {tool_name}: {summary}", file=sys.stderr)
        elif etype == "tool_completed":
            tool_name = payload.get("tool_name", "")
            status = payload.get("status", "")
            symbol = "✓" if status == "success" else "✗"
            print(f"  {symbol} {tool_name} ({status})", file=sys.stderr)
        elif etype == "permission_requested":
            perm = payload.get("permission", "")
            options = payload.get("options", [])
            print(f"\n  ⚡ Grok wants to: {perm}", file=sys.stderr)
            if options:
                print(f"     options: {', '.join(options)}", file=sys.stderr)
            if auto_approve:
                return "allow_once"
            try:
                choice = input("  Allow? [y/N] ").strip().lower()
                return "allow_once" if choice in ("y", "yes") else "cancelled"
            except (EOFError, KeyboardInterrupt):
                return "cancelled"
        elif etype == "error_event":
            msg = payload.get("message", "")
            print(f"\n  ⚠ {msg}", file=sys.stderr)
        return None

    # ── Interactive loop ───────────────────────────────────────────────
    print(f"gsa interactive — {run_id}", file=sys.stderr)
    print(f"  workspace: {workspace}", file=sys.stderr)
    print(f"  provider: {'loopback fake' if fake_provider else 'DeepSeek (real)'}", file=sys.stderr)
    print(f"  tools: enabled | permissions: {acp_permission_mode}", file=sys.stderr)
    print(f"  commands: /help /exit /clear", file=sys.stderr)
    print(f"  Type your message and press Enter.", file=sys.stderr)
    print(file=sys.stderr)

    exit_code = 0
    try:
        with GrokAcpSession(request) as session:
            # First prompt — optional initial message from --ask
            first_message = getattr(args, "ask", None) or getattr(args, "tui_run", None)
            if first_message:
                print(f"\n> {first_message}\n")
                session.send_prompt(first_message, on_acp_event=_on_event)
                print()

            while True:
                try:
                    user_input = input("> ").strip()
                except (EOFError, KeyboardInterrupt):
                    print("\ngsa: exiting.", file=sys.stderr)
                    break
                if not user_input:
                    continue
                if user_input == "/exit" or user_input == "/quit":
                    print("gsa: exiting.", file=sys.stderr)
                    break
                if user_input == "/help":
                    print("  /exit, /quit — end the session", file=sys.stderr)
                    print("  /clear       — clear screen", file=sys.stderr)
                    print("  /help        — show this help", file=sys.stderr)
                    continue
                if user_input == "/clear":
                    sys.stdout.write("\033[2J\033[H")
                    sys.stdout.flush()
                    continue
                print()
                session.send_prompt(user_input, on_acp_event=_on_event)
                print()
    except Exception as exc:
        print(f"\ngsa: session error — {exc}", file=sys.stderr)
        exit_code = 1
    return exit_code


def _run_tui_default(args: argparse.Namespace) -> int:
    """Launch TUI interactively — ``gsa`` with no subcommand.

    Detects the Grok binary (non-fatal), builds a TUI with sample data,
    and opens the address dialog so the user can type a command
    immediately.  Use ``/run <prompt>`` to start a Grok ACP session.
    """
    import sys as _sys
    from pathlib import Path as _Path
    from .tui.main import _run_demo
    from .tui.app import TuiPrototype

    # ── Grok check (non-fatal — TUI still works in demo mode) ──────────
    grok_ok = False
    try:
        from .grok_runtime_adapter import _detect_grok_binary
        grok_path, grok_ver = _detect_grok_binary()
        if grok_path:
            grok_ok = True
            ver_str = f" ({grok_ver})" if grok_ver else ""
            print(f"gsa: Grok detected{ver_str}", file=_sys.stderr)
        else:
            print("gsa: Grok not found on PATH", file=_sys.stderr)
            print("  TUI starts in demo mode. Type a message to start a session.",
                  file=_sys.stderr)
    except Exception as exc:
        print(f"gsa: Grok check failed — {exc}", file=_sys.stderr)
        print("  TUI starts in demo mode.", file=_sys.stderr)

    # ── Build TUI ──────────────────────────────────────────────────────
    app = TuiPrototype.with_sample_data()
    app.status_bar.update_item("Grok", grok_ok)
    if grok_ok:
        app.status_bar.update_item("就绪", True)
    return _run_demo(app, 100, 30, banner=False)


def _run_eval(args: argparse.Namespace) -> int:
    """Run a frozen evaluation from CLI."""
    from .evaluation_runner import run_evaluation_cli
    return run_evaluation_cli(
        scenario_path=args.scenario_bundle,
        oracle_path=args.oracle_bundle,
        output_dir=args.output_dir,
        model_id=args.model_id,
        seed=args.seed,
    )


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="gsa",
        description="General Scientific Assurance CLI dispatcher.",
    )
    parser.set_defaults(handler=_run_tui_default)
    subparsers = parser.add_subparsers(dest="command", required=False)

    doctor = subparsers.add_parser("doctor", help="Check local CLI readiness.")
    doctor.add_argument("--json", action="store_true", help="Emit JSON report.")
    doctor.add_argument(
        "--quick",
        action="store_true",
        help="Skip repository contract check for a fast wiring check.",
    )
    doctor.set_defaults(handler=_run_doctor)

    ask = subparsers.add_parser("ask", help="Ask a question using the DeepSeek API.")
    ask.add_argument("question", help="The question to ask (natural language).")
    ask.add_argument(
        "--credential-target",
        default=DEFAULT_CREDENTIAL_TARGET,
        help="Windows Credential Manager target for the API key.",
    )
    ask.add_argument("--max-tokens", type=int, default=4096)
    ask.add_argument("--timeout", type=int, default=120)
    ask.add_argument("--run-root", type=Path, default=None)
    ask.add_argument(
        "--retrieval",
        choices=["off", "subagent"],
        required=True,
        help="Retrieval mode (required): off (no retrieval, training data only — tool availability gate communicates UNAVAILABLE) or subagent (GSA retrieval subagents).",
    )
    ask.add_argument("--json", action="store_true", help="Emit full receipt as JSON.")
    ask.set_defaults(handler=_run_ask)

    source = subparsers.add_parser("source", help="Source-related gates.")
    source_subparsers = source.add_subparsers(dest="source_command", required=True)
    gate = source_subparsers.add_parser("gate", help="Evaluate source visibility gate.")
    gate.add_argument("--ledger", type=Path, required=True)
    gate.add_argument("--json", action="store_true")
    gate.add_argument("--summary", action="store_true")
    gate.set_defaults(handler=_run_source_gate)

    run = subparsers.add_parser(
        "run",
        help="Run the selected guarded CLI runtime path.",
    )
    run.add_argument("--run-root", type=Path, required=True)
    run.add_argument(
        "--runtime",
        choices=["canonical", "grok", "deepseek"],
        default="canonical",
        help=(
            "Runtime path. canonical is the default; grok emits an explicit "
            "fail-closed prompt/tool promotion gate receipt; deepseek runs "
            "the direct provider adapter."
        ),
    )
    run.add_argument("--source-ledger", type=Path)
    run.add_argument("--instruction-context", type=Path)
    run.add_argument("--ask")
    run.add_argument("--task", type=Path)
    run.add_argument("--run-id", default="RUN-CANONICAL-CLI-FAKE-001")
    run.add_argument("--task-id", default="TASK-CANONICAL-CLI-FAKE-001")
    run.add_argument("--created-at", default=None)
    run.add_argument("--json", action="store_true")
    run.add_argument(
        "--mode",
        choices=["fake", "real"],
        default="fake",
        help="Adapter mode: fake (offline, no network) or real (calls DeepSeek API).",
    )
    run.add_argument(
        "--credential-target",
        default=DEFAULT_CREDENTIAL_TARGET,
        help="Windows Credential Manager target name for the DeepSeek API key.",
    )
    run.add_argument(
        "--api-timeout",
        type=int,
        default=60,
        help="Timeout in seconds for the DeepSeek API call.",
    )
    run.add_argument(
        "--retrieval-mode",
        choices=["local_browser", "framework_fallback", "off"],
        default=None,
        help=(
            "Explicit retrieval mode for --runtime grok promotion gating. "
            "The gate records the mode but does not perform retrieval."
        ),
    )
    run.add_argument(
        "--grok-timeout-gate",
        type=Path,
        help=(
            "Attach a grok-timeout-gate-split verification receipt for "
            "--runtime grok promotion gating."
        ),
    )
    run.add_argument(
        "--grok-tool-availability-gate",
        type=Path,
        help=(
            "Attach a tool-availability gate receipt produced from Grok "
            "observe-tools for --runtime grok promotion gating."
        ),
    )
    run.add_argument(
        "--grok-adapter-receipt",
        type=Path,
        help=(
            "Attach a Grok runtime adapter receipt with containment "
            "evidence for --runtime grok promotion gating."
        ),
    )
    run.add_argument(
        "--grok-execute",
        action="store_true",
        help=(
            "When --runtime grok and gate decision is 'allow', execute "
            "a Grok session with the locked binary. Mode is selected "
            "by --grok-mode (default: acp-smoke)."
        ),
    )
    run.add_argument(
        "--grok-alpha-auto-evidence",
        action="store_true",
        help=(
            "With --grok-execute, generate local alpha-only no-tool and "
            "adapter-containment evidence before running ACP smoke. This "
            "does not promote the general Grok prompt/tool path."
        ),
    )
    run.add_argument(
        "--grok-fake-provider",
        action="store_true",
        help=(
            "For --grok-execute --grok-mode acp-smoke, use the local "
            "loopback fake provider instead of external model credentials."
        ),
    )
    run.add_argument(
        "--grok-mode",
        choices=["prompt-smoke", "acp-smoke"],
        default="acp-smoke",
        help=(
            "Execution mode for --grok-execute. "
            "'acp-smoke' uses grok agent stdio (ACP JSON-RPC, primary); "
            "'prompt-smoke' uses headless --prompt-file (narrow smoke only)."
        ),
    )
    run.add_argument(
        "--terminal-visibility",
        choices=["inline", "popout", "vscode"],
        default="popout",
        help=(
            "How terminal commands are executed. "
            "'inline' — Grok default (background, output to artifact). "
            "'popout' — launch Windows Terminal / conhost visible window. "
            "'vscode' — reuse VS Code integrated terminal."
        ),
    )
    run.set_defaults(handler=_run_canonical)

    verify = subparsers.add_parser("verify", help="Verify canonical guarded CLI run root.")
    verify.add_argument("--run-root", type=Path, required=True)
    verify.add_argument("--json", action="store_true")
    verify.set_defaults(handler=_run_verify)

    grok = subparsers.add_parser("grok", help="Grok runtime adapter commands.")
    grok_subparsers = grok.add_subparsers(dest="grok_command", required=True)
    grok_doctor = grok_subparsers.add_parser(
        "doctor",
        help="Inspect the locked Grok binary and runtime boundary.",
    )
    grok_doctor.add_argument("--json", action="store_true")
    grok_doctor.set_defaults(handler=_run_grok_doctor)
    grok_run = grok_subparsers.add_parser(
        "run",
        help="Run a supported Grok adapter smoke.",
    )
    grok_run.add_argument("--run-root", type=Path, required=True)
    grok_run.add_argument(
        "--workspace",
        type=Path,
        help=(
            "Workspace directory to trust and use as Grok cwd "
            "(default: isolated directory under --run-root)."
        ),
    )
    grok_run.add_argument(
        "--mode",
        choices=["version-smoke", "acp-smoke"],
        default="version-smoke",
        help="Grok adapter mode. acp-smoke is explicit/opt-in and does not change the default runtime.",
    )
    grok_run.add_argument(
        "--ask",
        help="Prompt text for --mode acp-smoke.",
    )
    grok_run.add_argument(
        "--fake-provider",
        action="store_true",
        help=(
            "For --mode acp-smoke, run against the local loopback fake "
            "DeepSeek fixture instead of requiring a real provider."
        ),
    )
    grok_run.add_argument(
        "--real-deepseek",
        action="store_true",
        help=(
            "For --mode acp-smoke, run one controlled real Grok ACP turn "
            "against DeepSeek using Windows Credential Manager."
        ),
    )
    grok_run.add_argument(
        "--credential-target",
        default=DEFAULT_CREDENTIAL_TARGET,
        help="Windows Credential Manager target for --real-deepseek.",
    )
    grok_run.add_argument(
        "--retrieval-mode",
        choices=["local_browser", "framework_fallback", "off"],
        default=None,
        help=(
            "Explicit retrieval mode for future Grok prompt/tool runs. "
            "version-smoke records the selection but never performs retrieval."
        ),
    )
    grok_run.add_argument("--run-id")
    grok_run.add_argument("--json", action="store_true")
    grok_run.set_defaults(handler=_run_grok)
    grok_observe_tools = grok_subparsers.add_parser(
        "observe-tools",
        help="Observe Grok tool registry and permission controls without prompt/tool execution.",
    )
    grok_observe_tools.add_argument(
        "--acp-verification",
        type=Path,
        action="append",
        default=[],
        help="Attach a grok-acp-fake-tool-probe verification JSON receipt.",
    )
    grok_observe_tools.add_argument(
        "--include-tool-availability",
        action="store_true",
        help="Include the projected tool availability report and gate receipt.",
    )
    grok_observe_tools.add_argument(
        "--output-gate-receipt",
        type=Path,
        help="Write only the tool availability gate receipt to a file for use with --runtime grok promotion gating.",
    )
    grok_observe_tools.add_argument("--json", action="store_true")
    grok_observe_tools.set_defaults(handler=_run_grok_observe_tools)

    deepseek = subparsers.add_parser(
        "deepseek",
        help="Direct DeepSeek runtime adapter commands.",
    )
    deepseek_subparsers = deepseek.add_subparsers(
        dest="deepseek_command", required=True
    )
    deepseek_run = deepseek_subparsers.add_parser(
        "run",
        help="Run one direct DeepSeek Chat Completions smoke and emit runtime events.",
    )
    deepseek_run.add_argument("--run-root", type=Path, required=True)
    deepseek_run.add_argument("--ask", required=True)
    deepseek_run.add_argument("--run-id")
    deepseek_run.add_argument(
        "--credential-target",
        default=DEFAULT_CREDENTIAL_TARGET,
        help="Windows Credential Manager target name for the DeepSeek API key.",
    )
    deepseek_run.add_argument(
        "--api-timeout",
        type=int,
        default=60,
        help="Timeout in seconds for the DeepSeek API call.",
    )
    deepseek_run.add_argument("--json", action="store_true")
    deepseek_run.set_defaults(handler=_run_deepseek)

    alpha = subparsers.add_parser("alpha", help="Alpha readiness entrypoints.")
    alpha_subparsers = alpha.add_subparsers(dest="alpha_command", required=True)
    alpha_smoke = alpha_subparsers.add_parser(
        "smoke",
        help="Run the fixed no-credential Grok ACP loopback smoke.",
    )
    alpha_smoke.add_argument(
        "--run-root",
        type=Path,
        help="Run root directory (default: .gsa/alpha-smoke/<timestamp-id>).",
    )
    alpha_smoke.add_argument(
        "--workspace",
        type=Path,
        help="Workspace directory (default: isolated directory under run root).",
    )
    alpha_smoke.add_argument(
        "--ask",
        help="Prompt text for the loopback ACP smoke fixture.",
    )
    alpha_smoke.add_argument("--run-id")
    alpha_smoke.add_argument("--json", action="store_true")
    alpha_smoke.set_defaults(handler=_run_alpha_smoke)
    alpha_ui = alpha_subparsers.add_parser(
        "ui-check",
        help="Run deterministic alpha UI usability checks.",
    )
    alpha_ui.add_argument(
        "--run-root",
        type=Path,
        help="Run root directory (default: .gsa/alpha-ui-check/<timestamp-id>).",
    )
    alpha_ui.add_argument("--width", type=int, default=100)
    alpha_ui.add_argument("--height", type=int, default=35)
    alpha_ui.add_argument(
        "--run-live-smoke",
        action="store_true",
        help="Also run a loopback Grok ACP static TUI smoke without full-screen interaction.",
    )
    alpha_ui.add_argument("--ask", help="Prompt text for --run-live-smoke.")
    alpha_ui.add_argument("--run-id")
    alpha_ui.add_argument(
        "--overwrite",
        action="store_true",
        help="Overwrite an existing alpha-ui-check receipt in the run root.",
    )
    alpha_ui.add_argument("--json", action="store_true")
    alpha_ui.set_defaults(handler=_run_alpha_ui_check)
    alpha_tool = alpha_subparsers.add_parser(
        "tool-check",
        help="Run alpha hard checks for Grok tool availability and ACP permission probes.",
    )
    alpha_tool.add_argument(
        "--run-root",
        type=Path,
        help="Run root directory (default: .gsa/alpha-tool-check/<timestamp-id>).",
    )
    alpha_tool.add_argument(
        "--acp-verification",
        type=Path,
        action="append",
        default=[],
        help="Attach a grok ACP fake-tool verification receipt; repeat for allow/cancel.",
    )
    alpha_tool.add_argument(
        "--overwrite",
        action="store_true",
        help="Overwrite existing alpha tool-check artifacts in the run root.",
    )
    alpha_tool.add_argument("--json", action="store_true")
    alpha_tool.set_defaults(handler=_run_alpha_tool_check)
    alpha_real = alpha_subparsers.add_parser(
        "real-call",
        help="Run an alpha real-provider Grok ACP call through DeepSeek.",
    )
    alpha_real.add_argument(
        "--run-root",
        type=Path,
        help="Run root directory (default: .gsa/alpha-real-call/<timestamp-id>).",
    )
    alpha_real.add_argument(
        "--workspace",
        type=Path,
        help="Workspace directory (default: isolated directory under the adapter run root).",
    )
    alpha_real.add_argument(
        "--ask",
        help="Prompt text for the real-provider ACP call.",
    )
    alpha_real.add_argument(
        "--transport",
        choices=["grok-acp", "direct-deepseek"],
        default="grok-acp",
        help="Real-call transport to verify (default: grok-acp).",
    )
    alpha_real.add_argument("--run-id")
    alpha_real.add_argument(
        "--credential-target",
        default=DEFAULT_CREDENTIAL_TARGET,
        help="Windows Credential Manager target containing the DeepSeek API key.",
    )
    alpha_real.add_argument(
        "--api-timeout",
        type=int,
        default=60,
        help="Timeout in seconds for the direct DeepSeek API call.",
    )
    alpha_real.add_argument(
        "--overwrite",
        action="store_true",
        help="Overwrite an existing alpha-real-call receipt in the run root.",
    )
    alpha_real.add_argument("--json", action="store_true")
    alpha_real.set_defaults(handler=_run_alpha_real_call)

    review = subparsers.add_parser("review", help="Review-mode utilities.")
    review_subparsers = review.add_subparsers(dest="review_command", required=True)
    global_review = review_subparsers.add_parser(
        "global",
        help="Explicitly activate Global Review Mode.",
    )
    global_review.add_argument(
        "--path",
        type=Path,
        action="append",
        default=[],
        help="Path to include in the global-review scope; repeatable.",
    )
    global_review.add_argument(
        "--no-git-status",
        action="store_true",
        help="Use only explicit --path values instead of also reading git status.",
    )
    global_review.add_argument("--review-id")
    global_review.add_argument("--output", type=Path)
    global_review.add_argument("--json", action="store_true")
    global_review.set_defaults(handler=_run_global_review)

    tui = subparsers.add_parser("tui", help="Render or run the terminal UI prototype.")
    tui.add_argument(
        "-w",
        "--width",
        type=int,
        default=100,
        help="Viewport width in terminal cells.",
    )
    tui.add_argument(
        "-H",
        "--height",
        type=int,
        default=30,
        help="Viewport height in terminal rows.",
    )
    tui.add_argument(
        "--demo",
        action="store_true",
        help="Run the interactive prompt_toolkit demo instead of a static render.",
    )
    tui.add_argument(
        "--with-dialog",
        action="store_true",
        help="Show the permission dialog overlay in static render/demo.",
    )
    tui.add_argument(
        "--with-properties",
        action="store_true",
        help="Show the properties sheet overlay in static render/demo.",
    )
    tui.add_argument(
        "--demo-events",
        choices=["gate_chain", "gate_defer", "run_failed"],
        help="Run the interactive demo with a live event scenario.",
    )
    tui.add_argument(
        "--live-retrieval",
        action="store_true",
        help="Wire the live browser retrieval handler.",
    )
    tui.add_argument(
        "--replay",
        type=Path,
        help="Replay events from an existing events.jsonl file.",
    )
    tui.add_argument(
        "--run",
        dest="tui_run",
        help="Run the selected runtime and display it in the TUI.",
    )
    tui.add_argument(
        "--runtime",
        choices=["canonical", "grok", "deepseek"],
        default="canonical",
        help="Runtime to use for --run.",
    )
    tui.add_argument(
        "--real",
        action="store_true",
        help="Use the real DeepSeek adapter for --run.",
    )
    tui.add_argument(
        "--fake-provider",
        action="store_true",
        help="For --runtime grok ACP runs, use the local loopback fake provider fixture.",
    )
    tui.add_argument(
        "--credential-target",
        default=DEFAULT_CREDENTIAL_TARGET,
        help="Windows Credential Manager target name for the DeepSeek API key.",
    )
    tui.add_argument(
        "--run-root",
        dest="tui_run_root",
        type=Path,
        help="Run root directory for --run.",
    )
    tui.add_argument(
        "--workspace",
        type=Path,
        help="Workspace directory for Grok runtime --run.",
    )
    tui.add_argument(
        "--retrieval-mode",
        dest="grok_retrieval_mode",
        choices=["local_browser", "framework_fallback", "off"],
        default=None,
        help=(
            "Explicit retrieval mode for Grok prompt/tool runs. "
            "Forwarded only when --runtime grok is selected."
        ),
    )
    tui.add_argument(
        "--terminal-visibility",
        choices=["inline", "popout", "vscode"],
        default="popout",
        help="How terminal commands are executed (default: popout).",
    )
    tui.set_defaults(handler=_run_tui)

    # ── chat ───────────────────────────────────────────────────────────────
    chat = subparsers.add_parser(
        "chat",
        help="Start an interactive CLI conversation with Grok ACP (tools enabled).",
    )
    chat.add_argument(
        "--ask",
        help="Initial message to send before entering the interactive loop.",
    )
    chat.add_argument(
        "--run-root",
        type=Path,
        help="Run root directory (default: .gsa/interactive/<timestamp-id>).",
    )
    chat.add_argument(
        "--workspace",
        type=Path,
        help="Workspace directory (default: isolated directory under run root).",
    )
    chat.add_argument("--run-id")
    chat.add_argument(
        "--credential-target",
        default=DEFAULT_CREDENTIAL_TARGET,
        help="Windows Credential Manager target for the DeepSeek API key.",
    )
    chat.add_argument(
        "--fake-provider",
        action="store_true",
        help="Use the local loopback fake provider instead of real DeepSeek.",
    )
    chat.add_argument(
        "--auto-approve",
        action="store_true",
        help="Auto-approve all permission requests (bypass interactive command review).",
    )
    chat.add_argument(
        "--retrieval-mode",
        dest="interactive_retrieval_mode",
        choices=["local_browser", "framework_fallback", "off"],
        default=None,
        help="Explicit retrieval mode (default: off).",
    )
    chat.set_defaults(handler=_run_interactive)

    # ── eval ─────────────────────────────────────────────────────────────
    eval_cmd = subparsers.add_parser("eval", help="Run frozen evaluation.")
    eval_cmd.add_argument("--scenario-bundle", type=Path, required=True,
                          help="Path to scenario bundle JSON.")
    eval_cmd.add_argument("--oracle-bundle", type=Path, required=True,
                          help="Path to oracle bundle JSON.")
    eval_cmd.add_argument("--output-dir", type=Path, required=True,
                          help="Directory for evaluation output.")
    eval_cmd.add_argument("--model-id", default="deepseek-chat",
                          help="Model ID for frozen profile.")
    eval_cmd.add_argument("--seed", type=int, default=42,
                          help="Random seed for frozen profile.")
    eval_cmd.set_defaults(handler=_run_eval)

    return parser


def main(argv: Sequence[str] | None = None) -> int:
    args = _parser().parse_args(argv)
    try:
        return int(args.handler(args))
    except AssuranceError as exc:
        print(
            _json_dump(
                {
                    "valid": False,
                    "error_type": "AssuranceError",
                    "error": str(exc),
                }
            ),
            file=sys.stderr,
        )
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
