from __future__ import annotations

from dataclasses import dataclass
import json
import os
from pathlib import Path
import subprocess
import time
from typing import Any, Callable
import uuid

from .contracts import validate_contract
from .errors import AssuranceError
from .grok_event_normalizer import normalize_grok_runtime_receipt, write_grok_events_jsonl
from .utils import atomic_write_json, load_json, sha256_file, utc_now


ROOT = Path(__file__).resolve().parents[1]
ADAPTER_ID = "grok-runtime-adapter"
ADAPTER_VERSION = "0.1.0"
RECEIPT_SCHEMA = "grok-runtime-receipt-v0.1.schema.json"
CREATE_NO_WINDOW = getattr(subprocess, "CREATE_NO_WINDOW", 0)


@dataclass(frozen=True)
class GrokRuntimeConfig:
    repo_root: Path = ROOT
    release_metadata_path: Path = ROOT / "upstream" / "grok-build.lock.json"
    inspect_script_path: Path = ROOT / "scripts" / "inspect_grok_install.ps1"
    workspace_trust_script_path: Path = ROOT / "scripts" / "new_grok_workspace_trust_receipt.ps1"
    timeout_seconds: int = 30


@dataclass(frozen=True)
class GrokRunRequest:
    run_root: Path
    workspace_path: Path
    run_id: str = "RUN-GROK-RUNTIME-SMOKE-001"
    mode: str = "version_smoke"
    trust_decision: str = "restricted"


def _run_json_command(command: list[str], *, cwd: Path, timeout: int) -> dict[str, Any]:
    completed = subprocess.run(
        command,
        cwd=cwd,
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
    try:
        value = json.loads(completed.stdout)
    except json.JSONDecodeError as exc:
        raise AssuranceError(f"command did not return JSON: {exc}") from exc
    if not isinstance(value, dict):
        raise AssuranceError("command JSON output is not an object")
    return value


def inspect_grok_runtime(config: GrokRuntimeConfig | None = None) -> dict[str, Any]:
    cfg = config or GrokRuntimeConfig()
    return _run_json_command(
        [
            "powershell",
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            str(cfg.inspect_script_path),
            "-ReleaseMetadataPath",
            str(cfg.release_metadata_path),
        ],
        cwd=cfg.repo_root,
        timeout=cfg.timeout_seconds,
    )


def _workspace_trust(
    *,
    config: GrokRuntimeConfig,
    request: GrokRunRequest,
    output_path: Path,
) -> dict[str, Any]:
    return _run_json_command(
        [
            "powershell",
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            str(config.workspace_trust_script_path),
            "-WorkspacePath",
            str(request.workspace_path),
            "-ProjectRootPath",
            str(request.workspace_path),
            "-OutputPath",
            str(output_path),
            "-Decision",
            request.trust_decision,
            "-DecisionActor",
            ADAPTER_ID,
        ],
        cwd=config.repo_root,
        timeout=config.timeout_seconds,
    )


def _trust_launch_permitted(receipt: dict[str, Any]) -> bool:
    if receipt.get("trust_granted") is True:
        return True
    decision = receipt.get("decision")
    return isinstance(decision, dict) and decision.get("launch_permitted") is True


def _clean_environment(profile: Path, temp: Path) -> dict[str, str]:
    keep = (
        "SystemRoot",
        "WINDIR",
        "COMSPEC",
        "PATH",
        "PATHEXT",
        "OS",
        "PROCESSOR_ARCHITECTURE",
        "NUMBER_OF_PROCESSORS",
    )
    environment = {key: os.environ[key] for key in keep if key in os.environ}
    environment.update(
        {
            "HOME": str(profile),
            "USERPROFILE": str(profile),
            "APPDATA": str(profile),
            "LOCALAPPDATA": str(profile),
            "TEMP": str(temp),
            "TMP": str(temp),
            "GROK_MEMORY": "0",
            "GROK_WEB_FETCH": "0",
            "HTTP_PROXY": "http://127.0.0.1:1",
            "HTTPS_PROXY": "http://127.0.0.1:1",
            "ALL_PROXY": "http://127.0.0.1:1",
            "NO_PROXY": "127.0.0.1,localhost",
        }
    )
    return environment


def _ensure_empty_run_root(path: Path) -> None:
    if path.exists() and any(path.iterdir()):
        raise AssuranceError(f"run_root must be empty or absent: {path}")
    path.mkdir(parents=True, exist_ok=True)


def run_grok_headless_once(
    request: GrokRunRequest,
    config: GrokRuntimeConfig | None = None,
    *,
    popen_factory: Callable[..., subprocess.Popen[bytes]] = subprocess.Popen,
) -> dict[str, Any]:
    """Run the first product-side Grok adapter smoke.

    This slice intentionally executes only ``grok --version`` through the
    locked binary.  It proves adapter launch, isolated environment, workspace
    trust ordering, artifact capture, root-process exit, and no root-process
    residue.  It does not send a prompt or invoke a model.
    """
    cfg = config or GrokRuntimeConfig()
    if request.mode != "version_smoke":
        raise AssuranceError(f"unsupported Grok runtime mode: {request.mode}")
    _ensure_empty_run_root(request.run_root)
    if not request.workspace_path.exists():
        if request.workspace_path.resolve().parent == request.run_root.resolve():
            request.workspace_path.mkdir()
        else:
            raise AssuranceError(
                f"workspace_path must be an existing directory: {request.workspace_path}"
            )
    if not request.workspace_path.is_dir():
        raise AssuranceError(f"workspace_path must be an existing directory: {request.workspace_path}")

    created_at = utc_now()
    trust_path = request.run_root / "workspace-trust.json"
    inspection_path = request.run_root / "binary-inspection.json"
    stdout_path = request.run_root / "grok.stdout.log"
    stderr_path = request.run_root / "grok.stderr.log"
    receipt_path = request.run_root / "grok-runtime-receipt.json"
    events_path = request.run_root / "events.jsonl"
    profile = request.run_root / "profile"
    temp = request.run_root / "temp"
    profile.mkdir()
    temp.mkdir()

    inspection = inspect_grok_runtime(cfg)
    atomic_write_json(inspection_path, inspection)
    if inspection.get("valid") is not True:
        raise AssuranceError("locked Grok binary inspection is invalid")

    trust = _workspace_trust(config=cfg, request=request, output_path=trust_path)
    trust_launch_permitted = _trust_launch_permitted(trust)
    if trust.get("valid") is not True or not trust_launch_permitted:
        raise AssuranceError("workspace trust was not granted before Grok launch")

    binary_path = Path(str(inspection["binary_path"]))
    stdout_handle = stdout_path.open("wb")
    stderr_handle = stderr_path.open("wb")
    process = popen_factory(
        [str(binary_path), "--version"],
        cwd=request.workspace_path,
        env=_clean_environment(profile, temp),
        stdin=subprocess.DEVNULL,
        stdout=stdout_handle,
        stderr=stderr_handle,
        creationflags=CREATE_NO_WINDOW,
    )
    try:
        try:
            process.wait(timeout=cfg.timeout_seconds)
        except subprocess.TimeoutExpired as exc:
            raise AssuranceError("Grok version smoke did not exit before timeout") from exc
    finally:
        stdout_handle.close()
        stderr_handle.close()

    root_process_exited = process.poll() is not None
    no_residue_observed = root_process_exited
    stdout_text = stdout_path.read_text(encoding="utf-8", errors="replace").strip()
    release = load_json(cfg.release_metadata_path)
    release_binary = release["binary_release"]
    expected_version_prefix = (
        f"grok {release_binary['version']} ({release_binary['build_id']})"
    )
    version_output_matches_lock = (
        stdout_text == expected_version_prefix
        or stdout_text.startswith(f"{expected_version_prefix} ")
    )
    checks = {
        "binary_inspection_valid": inspection.get("valid") is True,
        "workspace_trust_valid": trust.get("valid") is True,
        "workspace_trust_granted": trust_launch_permitted,
        "root_process_exited": root_process_exited,
        "no_residue_observed": no_residue_observed,
        "exit_code_zero": process.returncode == 0,
        "version_output_matches_lock": version_output_matches_lock,
    }
    receipt = {
        "schema_version": "0.1.0",
        "receipt_kind": "grok_runtime_adapter_receipt",
        "run_id": request.run_id,
        "created_at": created_at,
        "valid": all(checks.values()),
        "adapter": {
            "adapter_id": ADAPTER_ID,
            "adapter_version": ADAPTER_VERSION,
            "runtime_owner": "grok",
        },
        "request": {
            "mode": request.mode,
            "workspace_path": str(request.workspace_path.resolve()),
            "run_root": str(request.run_root.resolve()),
        },
        "binary": {
            "inspection_path": str(inspection_path.resolve()),
            "binary_path": str(binary_path.resolve()),
            "version_output": stdout_text,
            "sha256": inspection["observed"]["sha256"],
            "valid": inspection.get("valid") is True,
        },
        "workspace_trust": {
            "receipt_path": str(trust_path.resolve()),
            "valid": trust.get("valid") is True,
            "trust_granted": trust_launch_permitted,
            "aggregate_sha256": trust["discovery"]["aggregate_sha256"],
        },
        "execution": {
            "command_kind": "grok_version",
            "pid": int(process.pid),
            "exit_code": process.returncode,
            "timeout_seconds": cfg.timeout_seconds,
            "stdout_path": str(stdout_path.resolve()),
            "stderr_path": str(stderr_path.resolve()),
            "stdout_sha256": sha256_file(stdout_path),
            "stderr_sha256": sha256_file(stderr_path),
        },
        "containment": {
            "no_residue_required": True,
            "no_residue_observed": no_residue_observed,
            "root_process_exited": root_process_exited,
            "external_cleanup_required": False,
            "residue_scan_scope": "root_process_only",
        },
        "artifacts": {
            "receipt_path": str(receipt_path.resolve()),
            "events_path": str(events_path.resolve()),
        },
        "checks": checks,
        "limitations": [
            "This first adapter slice runs only grok --version; it does not send a prompt or model request.",
            "No-residue observation covers the supervised root process for this smoke path, not arbitrary tool child trees.",
            "Future prompt/tool modes must use stronger Windows Job Object containment before promotion.",
            "Captured stdout/stderr are metadata-bound artifacts; no raw prompt, hidden reasoning, or authorization material is recorded.",
        ],
    }
    validate_contract(receipt, RECEIPT_SCHEMA, label="Grok runtime receipt")
    atomic_write_json(receipt_path, receipt)
    events = normalize_grok_runtime_receipt(receipt, created_at=created_at)
    write_grok_events_jsonl(events_path, events)
    return receipt


def run_grok_version_smoke(
    *,
    run_root: Path,
    workspace_path: Path | None = None,
    run_id: str | None = None,
    config: GrokRuntimeConfig | None = None,
) -> dict[str, Any]:
    chosen_run_id = run_id or f"RUN-GROK-RUNTIME-SMOKE-{uuid.uuid4().hex[:8].upper()}"
    chosen_workspace = workspace_path or (run_root / "workspace")
    return run_grok_headless_once(
        GrokRunRequest(
            run_root=run_root,
            workspace_path=chosen_workspace,
            run_id=chosen_run_id,
            mode="version_smoke",
        ),
        config=config,
    )
