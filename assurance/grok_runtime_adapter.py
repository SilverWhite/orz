from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum
import json
import os
from pathlib import Path
import subprocess
import sys
import time
from typing import Any, Callable
import uuid

from .contracts import validate_contract
from .errors import AssuranceError


class TerminalVisibility(str, Enum):
    """How terminal commands are executed.

    ``INLINE`` — Grok default (background execution, output to artifact).
    ``POPOUT`` — Launch Windows Terminal / conhost in a visible window.
    ``VSCODE`` — Reuse VS Code integrated terminal (only in extension mode).
    """
    INLINE = "inline"
    POPOUT = "popout"
    VSCODE = "vscode"
from .grok_event_normalizer import normalize_grok_runtime_receipt, write_grok_events_jsonl
from .job_object_supervisor import (
    CREATE_SUSPENDED,
    JobObjectSupervisor,
    _resume_main_thread,
    contained_run,
)
from .utils import atomic_write_json, load_json, sha256_bytes, sha256_file, utc_now


ROOT = Path(__file__).resolve().parents[1]
ADAPTER_ID = "grok-runtime-adapter"
ADAPTER_VERSION = "0.1.0"
RECEIPT_SCHEMA = "grok-runtime-receipt-v0.1.schema.json"
CREATE_NO_WINDOW = getattr(subprocess, "CREATE_NO_WINDOW", 0)
SUPPORTED_RETRIEVAL_MODES = ("local_browser", "framework_fallback", "off")
DEFAULT_RETRIEVAL_MODE = "off"
NO_TOOL_DISALLOWED_TOOLS = (
    "run_terminal_cmd,grep,read_file,search_replace,list_dir,"
    "web_search,web_fetch,todo_write,task,Agent"
)


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
    retrieval_mode: str = DEFAULT_RETRIEVAL_MODE
    retrieval_mode_explicit: bool = False
    prompt_text: str | None = None
    model_id: str = "lif-fake-deepseek"
    max_turns: int = 20
    mcp_servers: list[dict[str, Any]] = field(default_factory=list)
    terminal_visibility: TerminalVisibility = TerminalVisibility.POPOUT
    fake_provider: bool = False
    provider_environment: dict[str, str] = field(default_factory=dict, repr=False)
    model_config_toml: str | None = None
    disable_builtin_tools: bool = False
    # GAK-03: ACP permission handling mode.
    #   "deny_by_default"  — cancel all permissions when no user callback (safe default)
    #   "auto_allow_once"  — auto-approve allow_once when no callback (smoke/dev only)
    #   "interactive"      — require on_acp_event callback; fail if missing
    acp_permission_mode: str = "interactive"


def _run_json_command(command: list[str], *, cwd: Path, timeout: int) -> dict[str, Any]:
    completed = contained_run(
        command,
        cwd=cwd,
        text=True,
        encoding="utf-8",
        errors="replace",
        timeout=timeout,
    )
    if completed.returncode != 0:
        stderr_text = (
            completed.stderr.strip() if isinstance(completed.stderr, str) else ""
        )
        raise AssuranceError(
            f"command failed ({completed.returncode}): {stderr_text}"
        )
    try:
        value = json.loads(completed.stdout)
    except json.JSONDecodeError as exc:
        raise AssuranceError(f"command did not return JSON: {exc}") from exc
    if not isinstance(value, dict):
        raise AssuranceError("command JSON output is not an object")
    return value


def launch_popout_terminal(command: str, *, cwd: str | None = None) -> subprocess.Popen[Any]:
    """Launch *command* in a visible Windows Terminal window.

    Uses ``Start-Process`` to open a new ``wt.exe`` (Windows Terminal) or
    ``powershell.exe`` window.  The caller receives the ``Popen`` handle
    but the window is owned by the user — closing it is their choice.

    Returns the ``Popen`` for the launcher process (which exits immediately
    after spawning the window).  On non-Windows, raises ``OSError``.
    """
    import shutil as _shutil
    import platform as _platform
    if _platform.system() != "Windows":
        raise OSError("POPOUT terminal visibility is Windows-only")

    wt_path = _shutil.which("wt.exe")
    if wt_path:
        popen = subprocess.Popen(
            ["wt.exe", "powershell", "-NoExit", "-Command", command],
            cwd=cwd, creationflags=subprocess.CREATE_NEW_CONSOLE if hasattr(subprocess, "CREATE_NEW_CONSOLE") else 0,
        )
    else:
        popen = subprocess.Popen(
            ["powershell", "-NoExit", "-Command", command],
            cwd=cwd, creationflags=subprocess.CREATE_NEW_CONSOLE if hasattr(subprocess, "CREATE_NEW_CONSOLE") else 0,
        )
    return popen


def _detect_grok_binary() -> tuple[str | None, str | None]:
    """Find the Grok binary on PATH and return (path, version).

    Returns (None, None) if Grok is not available.  This is a
    lightweight Python-only fallback for the PowerShell inspect script.
    """
    import shutil as _shutil
    grok_path = _shutil.which("grok")
    if not grok_path:
        return None, None
    try:
        result = subprocess.run(
            [grok_path, "--version"],
            capture_output=True, text=True, timeout=10,
        )
        if result.returncode != 0:
            return grok_path, None
        # Parse "grok 0.2.118 (1e1687c1cf)" from stdout or stderr.
        import re as _re
        match = _re.search(r"grok\s+(\S+)", result.stdout or result.stderr or "")
        version = match.group(1) if match else None
        return grok_path, version
    except (subprocess.TimeoutExpired, OSError):
        return grok_path, None


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


def validate_grok_retrieval_mode(mode: str) -> str:
    if mode not in SUPPORTED_RETRIEVAL_MODES:
        allowed = ", ".join(SUPPORTED_RETRIEVAL_MODES)
        raise AssuranceError(f"unsupported Grok retrieval mode: {mode} (allowed: {allowed})")
    return mode


def _clean_environment(
    profile: Path,
    temp: Path,
    *,
    extra: dict[str, str] | None = None,
) -> dict[str, str]:
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
    profile_path = str(profile.resolve())
    temp_path = str(temp.resolve())
    environment.update(
        {
            "HOME": profile_path,
            "USERPROFILE": profile_path,
            "APPDATA": profile_path,
            "LOCALAPPDATA": profile_path,
            "TEMP": temp_path,
            "TMP": temp_path,
            "GROK_MEMORY": "0",
            "GROK_WEB_FETCH": "0",
            "HTTP_PROXY": "http://127.0.0.1:1",
            "HTTPS_PROXY": "http://127.0.0.1:1",
            "ALL_PROXY": "http://127.0.0.1:1",
            "NO_PROXY": "127.0.0.1,localhost",
        }
    )
    if extra:
        environment.update(extra)
    return environment


def _write_loopback_fake_provider_config(profile: Path, *, port: int) -> Path:
    config_dir = profile.resolve() / ".grok"
    config_dir.mkdir(parents=True, exist_ok=True)
    config_path = config_dir / "config.toml"
    config = f"""
[features]
telemetry = false
feedback = false
lsp_tools = false
codebase_indexing = false
remote_fetch = false

[session]
load_envrc = false

[models]
default = "lif-fake-deepseek"
default_reasoning_effort = "high"

[model.lif-fake-deepseek]
model = "deepseek-v4-pro"
base_url = "http://127.0.0.1:{port}"
name = "LIF Fake DeepSeek Loopback"
env_key = "LIF_FAKE_DEEPSEEK_KEY"
api_backend = "chat_completions"
max_completion_tokens = 256
context_window = 4096

[permission]
rules = [
  {{ action = "deny", tool = "edit" }},
  {{ action = "deny", tool = "bash" }},
  {{ action = "deny", tool = "grep" }},
  {{ action = "deny", tool = "mcp" }},
  {{ action = "deny", tool = "webfetch" }},
  {{ action = "deny", tool = "websearch" }},
]
"""
    config_path.write_text(config.strip() + "\n", encoding="utf-8")
    return config_path


def _write_grok_model_config(
    profile: Path,
    config_toml: str,
    *,
    base_url_override: str | None = None,
) -> Path:
    config_dir = profile.resolve() / ".grok"
    config_dir.mkdir(parents=True, exist_ok=True)
    config_path = config_dir / "config.toml"
    text = config_toml.strip()
    if base_url_override:
        # Replace the hardcoded api.deepseek.com base_url with the proxy.
        text = text.replace(
            'base_url = "https://api.deepseek.com"',
            f'base_url = "{base_url_override}"',
        )
    config_path.write_text(text + "\n", encoding="utf-8")
    return config_path


def _grok_acp_command(binary_path: Path, request: GrokRunRequest) -> list[str]:
    command = [str(binary_path)]
    if request.disable_builtin_tools:
        command.extend(
            [
                "--disable-web-search",
                "--no-subagents",
                "--tools",
                "",
                "--disallowed-tools",
                NO_TOOL_DISALLOWED_TOOLS,
            ]
        )
    command.extend(["agent", "--model", request.model_id])
    if request.disable_builtin_tools:
        command.extend(["--reasoning-effort", "none"])
    command.append("stdio")
    return command


def _stderr_contains_provider_api_error(path: Path) -> bool:
    if not path.exists():
        return False
    try:
        text = path.read_text(encoding="utf-8", errors="replace")
    except OSError:
        return True
    return (
        "chat/completions API error" in text
        or "invalid_request_error" in text
        or "400 Bad Request" in text
    )


def _start_loopback_fake_provider(
    *,
    run_root: Path,
    timeout_seconds: int,
) -> tuple[subprocess.Popen[Any], dict[str, Any], Path, Path]:
    provider_root = run_root / "fake-provider"
    provider_script = ROOT / "scripts" / "fake_deepseek_provider.py"
    stdout_path = provider_root.with_name("fake-provider.stdout.log")
    stderr_path = provider_root.with_name("fake-provider.stderr.log")
    stdout_handle = stdout_path.open("wb")
    stderr_handle = stderr_path.open("wb")
    process = subprocess.Popen(
        [
            sys.executable,
            str(provider_script),
            "--output-directory",
            str(provider_root),
            "--timeout-seconds",
            str(timeout_seconds),
            "--scenario",
            "single",
        ],
        cwd=ROOT,
        stdout=stdout_handle,
        stderr=stderr_handle,
        stdin=subprocess.DEVNULL,
        creationflags=CREATE_NO_WINDOW,
    )
    ready_path = provider_root / "ready.json"
    deadline = time.monotonic() + min(max(timeout_seconds, 1), 30)
    ready: dict[str, Any] | None = None
    while time.monotonic() < deadline:
        if ready_path.is_file():
            candidate = load_json(ready_path)
            if (
                candidate.get("ready") is True
                and candidate.get("host") == "127.0.0.1"
                and candidate.get("external_bind") is False
                and isinstance(candidate.get("port"), int)
            ):
                ready = candidate
                break
        if process.poll() is not None:
            break
        time.sleep(0.05)
    stdout_handle.close()
    stderr_handle.close()
    if ready is None:
        try:
            process.kill()
        finally:
            process.wait(timeout=5)
        raise AssuranceError("loopback fake provider did not become ready")
    return process, ready, provider_root, ready_path


def _check_thinking_proxy_daemon() -> dict[str, Any] | None:
    """Return the daemon ready dict if the persistent proxy is running."""
    import socket as _socket

    daemon_dir = Path(os.environ.get("GSA_HOME", str(Path.home() / ".gsa"))) / "thinking-proxy"
    ready_path = daemon_dir / "daemon.json"
    if not ready_path.exists():
        return None
    try:
        ready = load_json(ready_path)
    except (OSError, json.JSONDecodeError):
        return None
    if not isinstance(ready, dict):
        return None
    if not ready.get("ready"):
        return None
    port = ready.get("port")
    if not isinstance(port, int):
        return None
    # Quick liveness check
    s = _socket.socket(_socket.AF_INET, _socket.SOCK_STREAM)
    s.settimeout(0.2)
    try:
        s.connect(("127.0.0.1", port))
        s.close()
        return ready
    except (OSError, ConnectionRefusedError):
        return None


def _ensure_empty_run_root(path: Path) -> None:
    if path.exists() and any(path.iterdir()):
        raise AssuranceError(f"run_root must be empty or absent: {path}")
    path.mkdir(parents=True, exist_ok=True)


def _start_deepseek_thinking_proxy(
    *,
    run_root: Path,
    timeout_seconds: int,
) -> tuple[subprocess.Popen[Any], dict[str, Any], Path, Path]:
    """Start a local passthrough proxy that injects ``thinking: disabled``.

    Returns ``(process, ready_dict, proxy_root, ready_path)`` — same
    signature as :func:`_start_loopback_fake_provider` so callers can
    use the same cleanup pattern.
    """
    proxy_root = run_root / "thinking-proxy"
    proxy_script = ROOT / "scripts" / "deepseek_thinking_proxy.py"
    stdout_path = proxy_root.with_name("thinking-proxy.stdout.log")
    stderr_path = proxy_root.with_name("thinking-proxy.stderr.log")
    stdout_handle = stdout_path.open("wb")
    stderr_handle = stderr_path.open("wb")
    process = subprocess.Popen(
        [
            sys.executable,
            str(proxy_script),
            "--output-dir",
            str(proxy_root),
        ],
        cwd=ROOT,
        stdout=stdout_handle,
        stderr=stderr_handle,
        stdin=subprocess.DEVNULL,
        creationflags=CREATE_NO_WINDOW,
    )
    ready_path = proxy_root / "ready.json"
    deadline = time.monotonic() + min(max(timeout_seconds, 1), 30)
    ready: dict[str, Any] | None = None
    while time.monotonic() < deadline:
        if ready_path.is_file():
            candidate = load_json(ready_path)
            if (
                isinstance(candidate.get("port"), int)
                and candidate.get("host") == "127.0.0.1"
                and candidate.get("proxy_type") == "thinking_disabled_passthrough"
            ):
                ready = candidate
                break
        if process.poll() is not None:
            break
        time.sleep(0.05)
    stdout_handle.close()
    stderr_handle.close()
    if ready is None:
        try:
            process.kill()
        finally:
            process.wait(timeout=5)
        raise AssuranceError("DeepSeek thinking proxy did not become ready")
    return process, ready, proxy_root, ready_path


def _parse_streaming_json_output(
    stdout: str,
) -> tuple[str, str, int]:
    """Parse Grok streaming-json output into (response_text, finish_reason, token_count).

    Grok ``--output-format streaming-json`` produces newline-delimited JSON
    objects with ``type`` fields: ``text`` (incremental text), ``end``
    (terminal with finish reason and usage), ``thought`` (reasoning —
    captured but not included in response_text).
    """
    response_parts: list[str] = []
    finish_reason = ""
    token_count = 0

    for line in stdout.splitlines():
        line = line.strip()
        if not line:
            continue
        try:
            obj = json.loads(line)
        except json.JSONDecodeError:
            continue
        t = obj.get("type", "")
        if t == "text":
            response_parts.append(obj.get("text", ""))
        elif t == "end":
            finish_reason = obj.get("finish_reason", "")
            usage = obj.get("usage", {})
            if isinstance(usage, dict):
                token_count = usage.get("total_tokens", 0)
        # "thought" events are intentionally discarded — they may contain
        # internal reasoning that should not appear in the receipt.

    return "".join(response_parts), finish_reason, token_count



def run_grok_headless_once(
    request: GrokRunRequest,
    config: GrokRuntimeConfig | None = None,
    *,
    popen_factory: Callable[..., subprocess.Popen[bytes]] = subprocess.Popen,
) -> dict[str, Any]:
    """Run a single Grok headless invocation through the locked binary.

    Supported modes:

    * ``version_smoke`` — ``grok --version`` (proves binary identity,
      workspace trust, environment isolation, containment, no residue).
    * ``prompt_smoke`` — ``grok --prompt-file <path> --output-format
      streaming-json`` (proves the full prompt→response chain with
      model invocation, streaming-json output capture, token counting).
    """
    cfg = config or GrokRuntimeConfig()
    if request.mode not in ("version_smoke", "prompt_smoke"):
        raise AssuranceError(f"unsupported Grok runtime mode: {request.mode}")
    retrieval_mode = validate_grok_retrieval_mode(request.retrieval_mode)
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

    if request.mode == "prompt_smoke" and not request.prompt_text:
        raise AssuranceError("prompt_text is required for mode 'prompt_smoke'")
    if request.mode == "version_smoke" and request.prompt_text:
        raise AssuranceError("prompt_text must not be set for mode 'version_smoke'")

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

    containment_diag: dict[str, Any] = {
        "job_object_created": False,
        "job_object_assigned": False,
        "containment_provider": "none",
        "containment_available": False,
    }

    supervisor: JobObjectSupervisor | None = None
    try:
        supervisor = JobObjectSupervisor()
        containment_diag["job_object_created"] = supervisor.is_active
        containment_diag["containment_available"] = supervisor.is_active
        if supervisor.is_active:
            containment_diag["containment_provider"] = "adapter_job_object"
    except AssuranceError:
        containment_diag["job_object_created"] = False
        containment_diag["containment_available"] = False
        raise

    # Build command args based on mode.
    prompt_path: Path | None = None
    if request.mode == "version_smoke":
        cmd = [str(binary_path), "--version"]
    else:  # prompt_smoke
        prompt_path = request.run_root / "prompt.txt"
        prompt_path.write_text(request.prompt_text or "", encoding="utf-8")
        cmd = [
            str(binary_path),
            "--prompt-file", str(prompt_path),
            "--output-format", "streaming-json",
            "--max-turns", str(request.max_turns),
            "--model", request.model_id,
            "--no-memory",
            "--sandbox", "read-only",
            "--permission-mode", "bypassPermissions",
            "--tools", "",
        ]

    process = popen_factory(
        cmd,
        cwd=request.workspace_path,
        env=_clean_environment(profile, temp),
        stdin=subprocess.DEVNULL,
        stdout=stdout_handle,
        stderr=stderr_handle,
        creationflags=CREATE_NO_WINDOW | CREATE_SUSPENDED,
    )
    try:
        if supervisor is not None and supervisor.is_active:
            supervisor.assign_process(process.pid)
            containment_diag["job_object_assigned"] = supervisor.is_assigned
            _resume_main_thread(process.pid)

        try:
            process.wait(timeout=cfg.timeout_seconds)
        except subprocess.TimeoutExpired as exc:
            raise AssuranceError(
                "Grok version smoke did not exit before timeout"
            ) from exc
    finally:
        stdout_handle.close()
        stderr_handle.close()
        if supervisor is not None:
            supervisor.close()

    root_process_exited = process.poll() is not None
    no_residue_observed = root_process_exited
    stdout_text = stdout_path.read_text(encoding="utf-8", errors="replace").strip()
    release = load_json(cfg.release_metadata_path)
    release_binary = release["binary_release"]

    # Mode-specific validation.
    is_prompt_smoke = request.mode == "prompt_smoke"
    version_output_matches_lock = False
    prompt_output_valid = False
    response_text = ""
    response_finish_reason = ""
    response_token_count = 0
    prompt_sha256 = ""

    if not is_prompt_smoke:
        expected_version_prefix = (
            f"grok {release_binary['version']} ({release_binary['build_id']})"
        )
        version_output_matches_lock = (
            stdout_text == expected_version_prefix
            or stdout_text.startswith(f"{expected_version_prefix} ")
        )
    else:
        prompt_sha256 = sha256_bytes(
            (request.prompt_text or "").encode("utf-8")
        )
        response_text, response_finish_reason, response_token_count = (
            _parse_streaming_json_output(stdout_text)
        )
        prompt_output_valid = (
            process.returncode == 0 and bool(response_text.strip())
        )

    checks = {
        "binary_inspection_valid": inspection.get("valid") is True,
        "workspace_trust_valid": trust.get("valid") is True,
        "workspace_trust_granted": trust_launch_permitted,
        "root_process_exited": root_process_exited,
        "no_residue_observed": no_residue_observed,
        "exit_code_zero": process.returncode == 0,
        "version_output_matches_lock": version_output_matches_lock,
        "prompt_output_valid": prompt_output_valid if is_prompt_smoke else True,
        "adapter_containment_available": containment_diag[
            "containment_available"
        ],
        "adapter_containment_provided": containment_diag[
            "job_object_assigned"
        ],
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
            "retrieval_mode": retrieval_mode,
            "retrieval_mode_explicit": request.retrieval_mode_explicit,
            "workspace_path": str(request.workspace_path.resolve()),
            "run_root": str(request.run_root.resolve()),
        },
        "retrieval": {
            "mode": retrieval_mode,
            "selected_explicitly": request.retrieval_mode_explicit,
            "applies_to": "prompt_tool_runs",
            "active_for_current_mode": request.mode != "version_smoke",
            "runtime_tool_retrieval_allowed": retrieval_mode != "off",
            "assurance_receipts_required": retrieval_mode != "off",
            "valid_modes": list(SUPPORTED_RETRIEVAL_MODES),
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
            "command_kind": "grok_version" if not is_prompt_smoke else "grok_prompt",
            "pid": int(process.pid),
            "exit_code": process.returncode,
            "timeout_seconds": cfg.timeout_seconds,
            "stdout_path": str(stdout_path.resolve()),
            "stderr_path": str(stderr_path.resolve()),
            "stdout_sha256": sha256_file(stdout_path),
            "stderr_sha256": sha256_file(stderr_path),
        },
        "prompt": (
            {
                "prompt_sha256": prompt_sha256,
                "prompt_bytes": len(request.prompt_text or ""),
                "model_id": request.model_id,
                "max_turns": request.max_turns,
                "response_summary": response_text[:200] if response_text else "",
                "response_sha256": (
                    sha256_bytes(response_text.encode("utf-8"))
                    if response_text else ""
                ),
                "response_finish_reason": response_finish_reason,
                "response_token_count": response_token_count,
                "output_format": "streaming_json",
            }
            if is_prompt_smoke
            else None
        ),
        "containment": {
            "no_residue_required": True,
            "no_residue_observed": no_residue_observed,
            "root_process_exited": root_process_exited,
            "external_cleanup_required": False,
            "residue_scan_scope": (
                "job_object_contained"
                if containment_diag["job_object_assigned"]
                else "root_process_only"
            ),
            "job_object_created": containment_diag["job_object_created"],
            "job_object_assigned": containment_diag["job_object_assigned"],
            "containment_provider": containment_diag["containment_provider"],
            "containment_available": containment_diag["containment_available"],
        },
        "artifacts": {
            "receipt_path": str(receipt_path.resolve()),
            "events_path": str(events_path.resolve()),
        },
        "checks": checks,
        "limitations": (
            [
                "Prompt smoke: first model-invocation slice — only text responses captured (no tool calls, no ACP).",
                "Adapter-side Kill-On-Close Job Object containment is active via JobObjectSupervisor (CREATE_SUSPENDED + AssignProcessToJobObject — post-creation race window closed). 2026-07-31 judgment: containment sufficient for prompt/tool promotion; PROC_THREAD_ATTRIBUTE_JOB_LIST is a code-quality refinement, not a security prerequisite.",
                "Retrieval mode is plumbed but prompt_smoke uses bypassPermissions with empty tools — retrieval is inactive.",
                "Captured stdout/stderr are metadata-bound artifacts; raw prompt content is hashed, not stored in receipt.",
            ]
            if is_prompt_smoke
            else [
                "This first adapter slice runs only grok --version; it does not send a prompt or model request.",
                "Adapter-side Kill-On-Close Job Object containment is active via JobObjectSupervisor (CREATE_SUSPENDED + AssignProcessToJobObject — post-creation race window closed). 2026-07-31 judgment: containment sufficient for prompt/tool promotion; PROC_THREAD_ATTRIBUTE_JOB_LIST is a code-quality refinement, not a security prerequisite.",
                "Retrieval mode is plumbed for future Grok prompt/tool runs only; version-smoke never performs retrieval.",
                "Captured stdout/stderr are metadata-bound artifacts; no raw prompt, hidden reasoning, or authorization material is recorded.",
            ]
        ),
    }
    validate_contract(receipt, RECEIPT_SCHEMA, label="Grok runtime receipt")
    atomic_write_json(receipt_path, receipt)
    events = normalize_grok_runtime_receipt(receipt, created_at=created_at)
    write_grok_events_jsonl(events_path, events)
    return receipt


def _append_permission_record_jsonl(path: Path, record: dict[str, Any]) -> None:
    """Append a single permission record dict as a JSONL line (D3.22)."""
    import json as _json
    line = _json.dumps(
        record, ensure_ascii=False, sort_keys=True, separators=(",", ":"), allow_nan=False,
    )
    with path.open("a", encoding="utf-8") as handle:
        handle.write(line + "\n")


class GrokAcpSession:
    """A live ACP session over ``grok agent stdio`` that supports multiple prompts.

    Usage::

        with GrokAcpSession(request, config) as session:
            result1 = session.send_prompt("first question", on_acp_event=handler)
            result2 = session.send_prompt("follow-up", on_acp_event=handler)
            # ... more prompts on the same sessionId ...
        receipt = session.receipt  # available after close
    """

    def __init__(
        self,
        request: GrokRunRequest,
        config: GrokRuntimeConfig | None = None,
        *,
        popen_factory: Callable[..., subprocess.Popen[bytes]] = subprocess.Popen,
    ) -> None:
        if request.mode != "acp_smoke":
            raise AssuranceError(f"GrokAcpSession requires mode 'acp_smoke', got {request.mode}")
        if not request.prompt_text:
            raise AssuranceError("prompt_text is required for acp_smoke")
        if request.fake_provider and request.model_config_toml:
            raise AssuranceError("fake_provider and model_config_toml are mutually exclusive")
        # GAK-03: validate acp_permission_mode
        _VALID_PERMISSION_MODES = frozenset({
            "deny_by_default", "auto_allow_once", "interactive",
        })
        if request.acp_permission_mode not in _VALID_PERMISSION_MODES:
            raise AssuranceError(
                f"acp_permission_mode must be one of {sorted(_VALID_PERMISSION_MODES)}, "
                f"got {request.acp_permission_mode!r}"
            )

        cfg = config or GrokRuntimeConfig()
        self._cfg = cfg
        self._request = request
        self._popen_factory = popen_factory
        self._created_at = utc_now()

        # ── Pre-flight ──────────────────────────────────────────────────
        retrieval_mode = validate_grok_retrieval_mode(request.retrieval_mode)
        self._retrieval_mode = retrieval_mode
        _ensure_empty_run_root(request.run_root)
        if not request.workspace_path.is_dir():
            request.workspace_path.mkdir(parents=True, exist_ok=True)

        self.run_root = request.run_root
        trust_path = request.run_root / "workspace-trust.json"
        inspection_path = request.run_root / "binary-inspection.json"
        self._receipt_path = request.run_root / "grok-runtime-receipt.json"
        self._events_path = request.run_root / "events.jsonl"
        self._acp_transcript_path = request.run_root / "acp_transcript.jsonl"
        self._stderr_path = request.run_root / "grok.stderr.log"
        self._permission_records_path = request.run_root / "permission-records.jsonl"
        profile = request.run_root / "profile"
        temp = request.run_root / "temp"
        profile.mkdir()
        temp.mkdir()
        self._fake_provider_process: subprocess.Popen[Any] | None = None
        self._fake_provider_ready: dict[str, Any] | None = None
        self._fake_provider_root: Path | None = None
        self._fake_provider_ready_path: Path | None = None
        self._fake_provider_config_path: Path | None = None
        self._thinking_proxy_process: subprocess.Popen[Any] | None = None
        self._thinking_proxy_ready: dict[str, Any] | None = None
        self._thinking_proxy_root: Path | None = None
        self._model_config_path: Path | None = None

        from .finding_registry import FindingRegistry, record_permission_decision
        self._finding_registry = FindingRegistry()

        inspection = inspect_grok_runtime(cfg)
        atomic_write_json(inspection_path, inspection)
        if inspection.get("valid") is not True:
            raise AssuranceError("locked Grok binary inspection is invalid")

        trust = _workspace_trust(config=cfg, request=request, output_path=trust_path)
        trust_launch_permitted = _trust_launch_permitted(trust)
        if trust.get("valid") is not True or not trust_launch_permitted:
            raise AssuranceError("workspace trust was not granted before Grok launch")

        self._binary_path = Path(str(inspection["binary_path"]))
        self._trust = trust
        self._trust_launch_permitted = trust_launch_permitted
        self._inspection = inspection
        self._inspection_path = inspection_path
        self._trust_path = trust_path

        # ── Containment ─────────────────────────────────────────────────
        self._containment_diag: dict[str, Any] = {
            "job_object_created": False, "job_object_assigned": False,
            "containment_provider": "none", "containment_available": False,
        }
        self._supervisor: JobObjectSupervisor | None = None
        try:
            self._supervisor = JobObjectSupervisor()
            self._containment_diag["job_object_created"] = self._supervisor.is_active
            self._containment_diag["containment_available"] = self._supervisor.is_active
            if self._supervisor.is_active:
                self._containment_diag["containment_provider"] = "adapter_job_object"
        except AssuranceError:
            raise

        extra_environment: dict[str, str] = dict(request.provider_environment)
        # ── Thinking proxy for real provider (GAK-DS-001) ──────────────
        # First check if the persistent daemon is already running (the
        # preferred path for Grok native compatibility).  Only start a
        # per-session foreground proxy if the daemon is absent.
        _proxy_base_url: str | None = None
        if request.model_config_toml and not request.fake_provider:
            _daemon_ready = _check_thinking_proxy_daemon()
            if _daemon_ready is not None:
                _proxy_base_url = (
                    f"http://127.0.0.1:{_daemon_ready['port']}"
                )
                self._thinking_proxy_process = None  # daemon, not ours
                self._thinking_proxy_ready = _daemon_ready
                self._thinking_proxy_root = None
            else:
                (
                    self._thinking_proxy_process,
                    self._thinking_proxy_ready,
                    self._thinking_proxy_root,
                    _thinking_proxy_ready_path,
                ) = _start_deepseek_thinking_proxy(
                    run_root=request.run_root,
                    timeout_seconds=cfg.timeout_seconds,
                )
            _proxy_base_url = (
                f"http://127.0.0.1:{self._thinking_proxy_ready['port']}"
            )
        if request.model_config_toml:
            self._model_config_path = _write_grok_model_config(
                profile,
                request.model_config_toml,
                base_url_override=_proxy_base_url,
            )
        if request.fake_provider:
            (
                self._fake_provider_process,
                self._fake_provider_ready,
                self._fake_provider_root,
                self._fake_provider_ready_path,
            ) = _start_loopback_fake_provider(
                run_root=request.run_root,
                timeout_seconds=cfg.timeout_seconds,
            )
            self._fake_provider_config_path = _write_loopback_fake_provider_config(
                profile,
                port=int(self._fake_provider_ready["port"]),
            )
            extra_environment["LIF_FAKE_DEEPSEEK_KEY"] = "loopback-fixture-not-a-secret"

        # ── Spawn grok agent stdio ──────────────────────────────────────
        self._stderr_handle = self._stderr_path.open("wb")
        self._process = popen_factory(
            _grok_acp_command(self._binary_path, request),
            cwd=request.workspace_path,
            env=_clean_environment(profile, temp, extra=extra_environment),
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=self._stderr_handle,
            text=True, encoding="utf-8", errors="replace", bufsize=1,
            creationflags=CREATE_NO_WINDOW | CREATE_SUSPENDED,
        )
        pid_for_containment = self._process.pid
        if self._supervisor is not None and self._supervisor.is_active:
            self._supervisor.assign_process(pid_for_containment)
            self._containment_diag["job_object_assigned"] = self._supervisor.is_assigned
            _resume_main_thread(pid_for_containment)

        # ── ACP state ───────────────────────────────────────────────────
        self.acp_events: list[dict[str, Any]] = []
        self.session_id: str = ""
        self.protocol_version: int = 0
        self.tool_call_count: int = 0
        self.permission_requests_count: int = 0
        self.permission_outcomes: list[str] = []
        self.turn_count: int = 0
        self.stop_reason: str = ""
        self.prompt_count: int = 0
        self._last_response_text: str = ""
        self._last_response_finish_reason: str = ""
        self._closed: bool = False
        self._last_streamed_text_len: int = 0  # P1.3: track cumulative text for delta emission
        self._last_streamed_text: str = ""
        self.receipt: dict[str, Any] | None = None

    # ── ACP wire helpers ──────────────────────────────────────────────────

    def _acp_send(self, msg: dict[str, Any]) -> None:
        if self._process.stdin is None or self._process.stdin.closed:
            raise AssuranceError("Grok ACP stdin closed unexpectedly")
        line = json.dumps(msg, ensure_ascii=False, separators=(",", ":"), allow_nan=False)
        self._process.stdin.write(line + "\n")
        self._process.stdin.flush()
        self.acp_events.append({"direction": "client_to_agent", "message": msg})

    def _acp_receive(self, timeout: float = 30.0) -> dict[str, Any]:
        import select as _sel
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            if self._process.poll() is not None:
                raise AssuranceError(
                    f"Grok ACP process exited early (code {self._process.returncode})"
                )
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise AssuranceError("timed out waiting for Grok ACP response")
            assert self._process.stdout is not None
            line = self._process.stdout.readline()
            if not line:
                raise AssuranceError("Grok ACP stdout closed unexpectedly")
            line = line.strip()
            if not line:
                continue
            try:
                msg = json.loads(line)
            except json.JSONDecodeError:
                raise AssuranceError(f"Grok ACP non-JSON output: {line[:200]}")
            if not isinstance(msg, dict):
                raise AssuranceError("Grok ACP non-object JSON output")
            self.acp_events.append({"direction": "agent_to_client", "message": msg})
            return msg
        raise AssuranceError("timed out waiting for Grok ACP response")

    def _acp_request(self, req_id: str, method: str, params: dict[str, Any]) -> dict[str, Any]:
        self._acp_send({"jsonrpc": "2.0", "id": req_id, "method": method, "params": params})
        while True:
            msg = self._acp_receive()
            if str(msg.get("id")) == req_id and "method" not in msg:
                if "error" in msg:
                    raise AssuranceError(f"ACP {method} error: {msg['error']}")
                result = msg.get("result")
                if not isinstance(result, dict):
                    raise AssuranceError(f"ACP {method} returned non-object result")
                return result
            self._handle_acp_notification(msg)

    def _handle_acp_notification(
        self, msg: dict[str, Any],
        on_acp_event: Callable[[dict[str, Any]], str | None] | None = None,
    ) -> None:
        # ── JSON-RPC error response (no method field) ──
        if "error" in msg and "method" not in msg:
            err = msg["error"]
            err_msg = err.get("message", str(err)) if isinstance(err, dict) else str(err)
            if on_acp_event:
                on_acp_event({
                    "event_type": "error_event", "timestamp": utc_now(),
                    "payload": {"message": str(err_msg)[:500], "source": "acp_rpc_error"},
                    "redaction": "metadata_only",
                })
            return

        method = msg.get("method", "")
        params = msg.get("params", {}) if isinstance(msg.get("params"), dict) else {}

        # ── Generic notification (ACP sends these alongside session/update) ──
        if method == "notification":
            notif_type = params.get("type", params.get("notificationType", ""))
            if notif_type == "error" or notif_type == "warning":
                if on_acp_event:
                    err_msg = params.get("message", params.get("error", str(params)))
                    on_acp_event({
                        "event_type": "error_event", "timestamp": utc_now(),
                        "payload": {
                            "message": str(err_msg)[:500],
                            "source": f"acp_{notif_type}",
                            "severity": "warning" if notif_type == "warning" else "error",
                        },
                        "redaction": "metadata_only",
                    })
            elif notif_type == "status":
                # Informational status — emit as status_update.
                if on_acp_event:
                    on_acp_event({
                        "event_type": "status_update", "timestamp": utc_now(),
                        "payload": {
                            "label": str(params.get("message", ""))[:100],
                            "ok": params.get("level", "info") != "error",
                        },
                        "redaction": "metadata_only",
                    })
            return

        if method in ("session/update", "_x.ai/session/update"):
            update = params.get("update", {}) if isinstance(params.get("update"), dict) else {}
            update_type = update.get("sessionUpdate", "")
            if update_type == "tool_call":
                self.tool_call_count += 1
                if on_acp_event:
                    tool_info = update.get("tool_call", {}) if isinstance(update.get("tool_call"), dict) else {}
                    on_acp_event({
                        "event_type": "tool_proposal", "timestamp": utc_now(),
                        "payload": {
                            "tool_name": tool_info.get("title", tool_info.get("name", "")),
                            "tool_call_id": tool_info.get("id", ""),
                            "input_summary": tool_info.get("description", ""),
                        },
                        "redaction": "metadata_only",
                    })
            elif update_type == "tool_call_update":
                if on_acp_event:
                    tool_info = update.get("tool_call", {}) if isinstance(update.get("tool_call"), dict) else {}
                    args_text = tool_info.get("arguments", "")
                    if isinstance(args_text, dict):
                        args_text = json.dumps(args_text, ensure_ascii=False)[:200]
                    elif isinstance(args_text, str) and len(args_text) > 200:
                        args_text = args_text[:200]
                    on_acp_event({
                        "event_type": "tool_proposal", "timestamp": utc_now(),
                        "payload": {
                            "tool_name": tool_info.get("title", tool_info.get("name", "")),
                            "tool_call_id": tool_info.get("id", ""),
                            "input_summary": str(args_text)[:200] if args_text else "",
                        },
                        "redaction": "metadata_only",
                    })
            elif update_type in (
                "assistant_message",
                "agent_message",
                "agent_message_chunk",
                "message",
                "user_message",
                "user_message_chunk",
            ):
                text = ""
                if isinstance(update.get("content"), list):
                    for block in update["content"]:
                        if isinstance(block, dict) and block.get("type") == "text":
                            text += block.get("text", "")
                elif isinstance(update.get("content"), dict):
                    content = update["content"]
                    if content.get("type") == "text":
                        text = content.get("text", "")
                elif isinstance(update.get("text"), str):
                    text = update["text"]
                elif isinstance(update.get("message"), dict):
                    text = update["message"].get("content", "")
                if text:
                    if update_type in ("user_message", "user_message_chunk"):
                        if on_acp_event:
                            on_acp_event({
                                "event_type": "user_message", "timestamp": utc_now(),
                                "payload": {"text": text, "turn": self.turn_count},
                                "redaction": "metadata_only",
                            })
                        return
                    # Emit only the incremental portion as text_delta (P1.3).
                    is_cumulative_update = len(text) >= self._last_streamed_text_len and (
                        self._last_streamed_text_len == 0
                        or text.startswith(getattr(self, "_last_streamed_text", ""))
                    )
                    if is_cumulative_update:
                        new_text = text[self._last_streamed_text_len:]
                    else:
                        new_text = text
                    if new_text:
                        if on_acp_event:
                            on_acp_event({
                                "event_type": "text_delta", "timestamp": utc_now(),
                                "payload": {"text": new_text, "turn": self.turn_count},
                                "redaction": "metadata_only",
                            })
                        if is_cumulative_update:
                            self._last_streamed_text = text
                            self._last_streamed_text_len = len(text)
                        else:
                            self._last_streamed_text = (
                                getattr(self, "_last_streamed_text", "") + new_text
                            )
                            self._last_streamed_text_len = len(self._last_streamed_text)
                        self._last_response_text = self._last_streamed_text
            elif update_type == "error":
                if on_acp_event:
                    error_msg = update.get("message", update.get("error", "unknown ACP error"))
                    on_acp_event({
                        "event_type": "error_event", "timestamp": utc_now(),
                        "payload": {
                            "message": str(error_msg)[:500],
                            "source": "acp_session_update",
                            "severity": "error",
                        },
                        "redaction": "metadata_only",
                    })
            elif update_type == "tool_result":
                if on_acp_event:
                    result_info = update.get("tool_result", {}) if isinstance(update.get("tool_result"), dict) else {}
                    status = "error" if result_info.get("is_error") else "success"
                    tool_name = result_info.get("title", result_info.get("name", ""))
                    on_acp_event({
                        "event_type": "tool_completed", "timestamp": utc_now(),
                        "payload": {
                            "tool_name": tool_name,
                            "status": status,
                            "output_sha256": sha256_bytes(
                                json.dumps(result_info, ensure_ascii=False, sort_keys=True,
                                           separators=(",", ":"), allow_nan=False).encode("utf-8")
                            ) if result_info else "",
                        },
                        "redaction": "metadata_only",
                    })
            elif update_type == "turn_completed":
                self.turn_count += 1
                self.stop_reason = update.get("stop_reason", self.stop_reason)
        elif method == "session/request_permission" and "id" in msg:
            self.permission_requests_count += 1
            options = params.get("options", [])
            option_labels = [o.get("kind", "") for o in options if isinstance(o, dict)]
            if on_acp_event:
                tool_name = ""
                if options:
                    first = options[0] if isinstance(options[0], dict) else {}
                    tool_name = first.get("toolTitle", first.get("title", ""))
                user_decision = on_acp_event({
                    "event_type": "permission_requested", "timestamp": utc_now(),
                    "payload": {"permission": tool_name, "options": option_labels},
                    "redaction": "metadata_only",
                })
            else:
                tool_name = ""
                user_decision = None
            allow_id = next(
                (o["optionId"] for o in options if isinstance(o, dict) and o.get("kind") == "allow_once"), None,
            )
            if user_decision == "allow_once" and allow_id is not None:
                self._acp_send({"jsonrpc": "2.0", "id": msg["id"],
                                "result": {"outcome": {"outcome": "selected", "optionId": allow_id}}})
                self.permission_outcomes.append("allow_once")
            elif user_decision == "allow_once" and allow_id is None:
                self._acp_send({"jsonrpc": "2.0", "id": msg["id"],
                                "result": {"outcome": {"outcome": "cancelled"}}})
                self.permission_outcomes.append("cancelled")
            elif isinstance(user_decision, str) and user_decision:
                self._acp_send({"jsonrpc": "2.0", "id": msg["id"],
                                "result": {"outcome": {"outcome": "cancelled"}}})
                self.permission_outcomes.append(user_decision)
            elif allow_id is not None:
                # GAK-03: no user callback — decision depends on acp_permission_mode
                if self._request.acp_permission_mode == "auto_allow_once":
                    self._acp_send({"jsonrpc": "2.0", "id": msg["id"],
                                    "result": {"outcome": {"outcome": "selected", "optionId": allow_id}}})
                    self.permission_outcomes.append("allow_once")
                elif self._request.acp_permission_mode == "interactive":
                    raise AssuranceError(
                        "ACP permission requested but no on_acp_event callback "
                        "is attached and acp_permission_mode is 'interactive'"
                    )
                else:
                    # deny_by_default: fail closed
                    self._acp_send({"jsonrpc": "2.0", "id": msg["id"],
                                    "result": {"outcome": {"outcome": "cancelled"}}})
                    self.permission_outcomes.append("cancelled")
            else:
                self._acp_send({"jsonrpc": "2.0", "id": msg["id"],
                                "result": {"outcome": {"outcome": "cancelled"}}})
                self.permission_outcomes.append("cancelled")
            decision_source = "user" if user_decision else (
                "adapter" if self._request.acp_permission_mode == "auto_allow_once"
                else "adapter-deny-by-default"
            )
            final_decision = self.permission_outcomes[-1] if self.permission_outcomes else "cancelled"
            try:
                from .finding_registry import record_permission_decision
                perm_record = record_permission_decision(
                    registry=self._finding_registry, session_run_id=self._request.run_id,
                    decision=final_decision, authority=decision_source,
                    tool_name=tool_name,
                    basis=(f"user decided '{final_decision}' via TUI dialog"
                           if decision_source == "user"
                           else f"adapter auto-decided '{final_decision}'"),
                )
                _append_permission_record_jsonl(self._permission_records_path, perm_record.to_dict())
            except Exception:
                pass
            if on_acp_event:
                on_acp_event({
                    "event_type": "permission_decision", "timestamp": utc_now(),
                    "payload": {
                        "permission": tool_name if tool_name else option_labels[0] if option_labels else "",
                        "decision": final_decision, "decision_source": decision_source,
                    },
                    "redaction": "metadata_only",
                })
        elif method == "_x.ai/session/prompt_complete":
            self.stop_reason = params.get("stopReason", self.stop_reason)
            if params.get("sessionId"):
                self.session_id = params["sessionId"]

    # ── Lifecycle ─────────────────────────────────────────────────────────

    def __enter__(self) -> GrokAcpSession:
        """ACP initialize + session/new handshake."""
        try:
            init_result = self._acp_request("acp-init-1", "initialize", {
                "protocolVersion": 1, "clientCapabilities": {},
                "clientInfo": {"name": "gsa-acp-adapter", "title": "GSA ACP Adapter", "version": "0.1.0"},
            })
            self.protocol_version = init_result.get("protocolVersion", 0)

            session_result = self._acp_request("acp-session-1", "session/new", {
                "cwd": str(self._request.workspace_path.resolve()),
                "mcpServers": self._request.mcp_servers,
            })
            self.session_id = session_result.get("sessionId", self.session_id)
            return self
        except Exception:
            self.close()
            raise

    def __exit__(self, *_: Any) -> None:
        self.close()

    def send_prompt(
        self,
        prompt_text: str,
        *,
        on_acp_event: Callable[[dict[str, Any]], str | None] | None = None,
    ) -> dict[str, Any]:
        """Send a new ``session/prompt`` on the existing session and return the result.

        Emits ``acp_initialize`` and ``acp_session_created`` only on the first
        call; subsequent calls skip those lifecycle events.
        """
        if self._closed:
            raise AssuranceError("GrokAcpSession is closed")
        self.prompt_count += 1
        self._last_streamed_text_len = 0  # P1.3: reset per-turn text tracker
        self._last_streamed_text = ""
        is_first = self.prompt_count == 1

        if is_first and on_acp_event:
            on_acp_event({
                "event_type": "acp_initialize", "timestamp": utc_now(),
                "payload": {"protocol_version": self.protocol_version},
                "redaction": "metadata_only",
            })
            on_acp_event({
                "event_type": "acp_session_created", "timestamp": utc_now(),
                "payload": {"session_id_hash": sha256_bytes(
                    self.session_id.encode("utf-8")) if self.session_id else ""},
                "redaction": "metadata_only",
            })

        self._acp_send({
            "jsonrpc": "2.0", "id": f"acp-prompt-{self.prompt_count}",
            "method": "session/prompt",
            "params": {
                "sessionId": self.session_id,
                "prompt": [{"type": "text", "text": prompt_text}],
                "maxTurns": self._request.max_turns,
            },
        })
        prompt_result: dict[str, Any] = {}
        while True:
            msg = self._acp_receive(timeout=self._cfg.timeout_seconds)
            if str(msg.get("id")) == f"acp-prompt-{self.prompt_count}" and "method" not in msg:
                if "error" in msg:
                    raise AssuranceError(f"session/prompt error: {msg['error']}")
                prompt_result = msg.get("result", {}) if isinstance(msg.get("result"), dict) else {}
                break
            self._handle_acp_notification(msg, on_acp_event=on_acp_event)

        # Extract final text from prompt_result.
        _stop_reason = prompt_result.get("stopReason", "")
        if _stop_reason:
            self.stop_reason = _stop_reason
            self._last_response_finish_reason = _stop_reason
        if prompt_result:
            _final_text = ""
            if isinstance(prompt_result.get("message"), dict):
                _msg = prompt_result["message"]
                if isinstance(_msg.get("content"), list):
                    for _block in _msg["content"]:
                        if isinstance(_block, dict) and _block.get("type") == "text":
                            _final_text += _block.get("text", "")
                elif isinstance(_msg.get("content"), str):
                    _final_text = _msg["content"]
            if _final_text:
                self._last_response_text = _final_text
                if on_acp_event:
                    on_acp_event({
                        "event_type": "model_output", "timestamp": utc_now(),
                        "payload": {
                            "text": _final_text, "turn": self.turn_count,
                            "stop_reason": self.stop_reason, "structured_output_valid": True,
                        },
                        "redaction": "metadata_only",
                    })

        # Drain remaining notifications.
        drain_deadline = time.monotonic() + 5.0
        while time.monotonic() < drain_deadline and self._process.poll() is None:
            assert self._process.stdout is not None
            import select as _sel
            try:
                ready, _, _ = _sel.select([self._process.stdout], [], [], 0.5)
            except (OSError, TypeError, ValueError):
                break
            if not ready:
                break
            line = self._process.stdout.readline()
            if not line:
                break
            line = line.strip()
            if not line:
                continue
            try:
                msg = json.loads(line)
            except json.JSONDecodeError:
                continue
            if isinstance(msg, dict):
                self._handle_acp_notification(msg, on_acp_event=on_acp_event)
                self.acp_events.append({"direction": "agent_to_client", "message": msg})

        return prompt_result

    def close(self) -> dict[str, Any]:
        """Close stdin, wait for the process, write receipts, and return the receipt dict."""
        if self._closed:
            return self.receipt if self.receipt is not None else {}
        self._closed = True

        if self._process.stdin is not None and not self._process.stdin.closed:
            self._process.stdin.close()
        try:
            self._process.wait(timeout=10)
        except subprocess.TimeoutExpired:
            self._process.kill()
            self._process.wait(timeout=5)
        finally:
            self._stderr_handle.close()
            if self._fake_provider_process is not None:
                try:
                    self._fake_provider_process.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    self._fake_provider_process.kill()
                    self._fake_provider_process.wait(timeout=5)
            if self._thinking_proxy_process is not None:
                # Only kill foreground proxies — never the daemon.
                try:
                    self._thinking_proxy_process.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    self._thinking_proxy_process.kill()
                    self._thinking_proxy_process.wait(timeout=5)
            if self._supervisor is not None:
                self._supervisor.close()

        # ── Write transcript ────────────────────────────────────────────
        transcript_lines = [
            json.dumps(
                {"direction": e["direction"],
                 "message_sha256": sha256_bytes(
                     json.dumps(e["message"], ensure_ascii=False, sort_keys=True,
                                separators=(",", ":"), allow_nan=False).encode("utf-8")
                 )},
                ensure_ascii=False, sort_keys=True, separators=(",", ":"), allow_nan=False,
            )
            for e in self.acp_events
        ]
        self._acp_transcript_path.write_text("\n".join(transcript_lines) + "\n", encoding="utf-8")

        # ── Build receipt ───────────────────────────────────────────────
        from .utils import sha256_file as _sha256_file, sha256_bytes as _sha256_bytes
        root_process_exited = self._process.poll() is not None
        no_residue_observed = root_process_exited
        version_output = str(
            self._inspection.get("observed", {}).get("version_output", "")
        )
        response_text = self._last_response_text
        provider_api_error_observed = bool(
            self._request.model_config_toml
            and _stderr_contains_provider_api_error(self._stderr_path)
        )
        checks = {
            "binary_inspection_valid": self._inspection.get("valid") is True,
            "workspace_trust_valid": self._trust.get("valid") is True,
            "workspace_trust_granted": self._trust_launch_permitted,
            "root_process_exited": root_process_exited,
            "no_residue_observed": no_residue_observed,
            "exit_code_zero": self._process.returncode == 0,
            "acp_initialize_ok": self.protocol_version >= 1,
            "acp_session_created": bool(self.session_id),
            "acp_prompt_completed": self.prompt_count > 0,
            "acp_response_received": bool(response_text.strip()),
            "adapter_containment_available": self._containment_diag["containment_available"],
            "adapter_containment_provided": self._containment_diag["job_object_assigned"],
            "provider_api_errors_absent": not provider_api_error_observed,
        }
        limitations = [
            "ACP session guarantees are bounded by the Grok process lifecycle and Job Object containment.",
            "Transcript records direction and message digest only — raw content is not persisted.",
            "Multi-prompt sessions accumulate state in the Grok process; no cross-session isolation beyond sessionId.",
        ]
        if self._request.fake_provider:
            limitations.append(
                "Loopback fake provider mode uses a fixed local SSE fixture and does not prove real DeepSeek compatibility or non-loopback firewall isolation."
            )
        elif self._request.model_config_toml:
            limitations.append(
                "Real provider mode permits external network only through the explicitly injected Grok model configuration and child-process environment."
            )
        receipt = {
            "schema_version": "0.1.0",
            "receipt_kind": "grok_runtime_adapter_receipt",
            "run_id": self._request.run_id,
            "created_at": self._created_at,
            "valid": all(checks.values()),
            "adapter": {"adapter_id": ADAPTER_ID, "adapter_version": ADAPTER_VERSION, "runtime_owner": "grok"},
            "request": {
                "mode": self._request.mode,
                "retrieval_mode": self._retrieval_mode,
                "retrieval_mode_explicit": self._request.retrieval_mode_explicit,
                "workspace_path": str(self._request.workspace_path.resolve()),
                "run_root": str(self._request.run_root.resolve()),
                "fake_provider": self._request.fake_provider,
                "disable_builtin_tools": self._request.disable_builtin_tools,
            },
            "retrieval": {
                "mode": self._retrieval_mode,
                "selected_explicitly": self._request.retrieval_mode_explicit,
                "applies_to": "prompt_tool_runs",
                "active_for_current_mode": self._retrieval_mode != "off",
                "runtime_tool_retrieval_allowed": self._retrieval_mode != "off",
                "assurance_receipts_required": self._retrieval_mode != "off",
                "valid_modes": list(SUPPORTED_RETRIEVAL_MODES),
            },
            "binary": {
                "inspection_path": str(self._inspection_path.resolve()),
                "binary_path": str(self._binary_path.resolve()),
                "version_output": version_output,
                "sha256": self._inspection["observed"]["sha256"],
                "valid": self._inspection.get("valid") is True,
            },
            "workspace_trust": {
                "receipt_path": str(self._trust_path.resolve()),
                "valid": self._trust.get("valid") is True,
                "trust_granted": self._trust_launch_permitted,
                "aggregate_sha256": self._trust["discovery"]["aggregate_sha256"],
            },
            "execution": {
                "command_kind": "grok_acp",
                "pid": int(self._process.pid),
                "exit_code": self._process.returncode,
                "timeout_seconds": self._cfg.timeout_seconds,
                "stdout_path": str(self._acp_transcript_path.resolve()),
                "stderr_path": str(self._stderr_path.resolve()),
                "stdout_sha256": _sha256_file(self._acp_transcript_path) if self._acp_transcript_path.exists() else "",
                "stderr_sha256": _sha256_file(self._stderr_path),
            },
            "fake_provider": (
                {
                    "enabled": True,
                    "ready_path": str(self._fake_provider_ready_path.resolve())
                    if self._fake_provider_ready_path else "",
                    "result_path": str((self._fake_provider_root / "provider-result.json").resolve())
                    if self._fake_provider_root else "",
                    "config_path": str(self._fake_provider_config_path.resolve())
                    if self._fake_provider_config_path else "",
                    "host": str((self._fake_provider_ready or {}).get("host", "")),
                    "port": int((self._fake_provider_ready or {}).get("port", 0)),
                    "real_model_invoked": False,
                }
                if self._request.fake_provider
                else None
            ),
            "prompt": {
                "prompt_sha256": _sha256_bytes((self._request.prompt_text or "").encode("utf-8")),
                "prompt_bytes": len(self._request.prompt_text or ""),
                "model_id": self._request.model_id,
                "max_turns": self._request.max_turns,
                "response_summary": response_text[:200] if response_text else "",
                "response_sha256": _sha256_bytes(response_text.encode("utf-8")),
                "response_finish_reason": self._last_response_finish_reason or self.stop_reason,
                "response_token_count": 0,
                "output_format": "acp_json_rpc",
            },
            "acp": {
                "protocol_version": self.protocol_version,
                "session_id_hash": _sha256_bytes(self.session_id.encode("utf-8")),
                "tool_call_count": self.tool_call_count,
                "permission_requests_count": self.permission_requests_count,
                "permission_outcomes": list(self.permission_outcomes),
                "acp_permission_mode": self._request.acp_permission_mode,
                "turn_count": self.turn_count,
                "stop_reason": self.stop_reason,
                "event_count": len(self.acp_events),
                "prompt_count": self.prompt_count,
            },
            "containment": {
                "no_residue_required": True,
                "no_residue_observed": no_residue_observed,
                "root_process_exited": root_process_exited,
                "external_cleanup_required": False,
                "residue_scan_scope": (
                    "job_object_contained"
                    if self._containment_diag["job_object_assigned"]
                    else "root_process_only"
                ),
                "job_object_created": self._containment_diag["job_object_created"],
                "job_object_assigned": self._containment_diag["job_object_assigned"],
                "containment_provider": self._containment_diag["containment_provider"],
                "containment_available": self._containment_diag["containment_available"],
            },
            "artifacts": {
                "receipt_path": str(self._receipt_path.resolve()),
                "events_path": str(self._events_path.resolve()),
                "acp_transcript_path": str(self._acp_transcript_path.resolve()),
                "stderr_path": str(self._stderr_path.resolve()),
                "binary_inspection_path": str(self._inspection_path.resolve()),
                "workspace_trust_path": str(self._trust_path.resolve()),
                "permission_records_path": str(self._permission_records_path.resolve()),
                "fake_provider_ready_path": str(self._fake_provider_ready_path.resolve())
                if self._fake_provider_ready_path else "",
                "fake_provider_result_path": str((self._fake_provider_root / "provider-result.json").resolve())
                if self._fake_provider_root else "",
            },
            "checks": checks,
            "limitations": limitations,
        }
        validate_contract(receipt, RECEIPT_SCHEMA, label="grok runtime receipt")
        events = normalize_grok_runtime_receipt(receipt, created_at=self._created_at)
        write_grok_events_jsonl(self._events_path, events)
        atomic_write_json(self._receipt_path, receipt)

        self.receipt = receipt
        return receipt


def run_grok_acp_once(
    request: GrokRunRequest,
    config: GrokRuntimeConfig | None = None,
    *,
    popen_factory: Callable[..., subprocess.Popen[bytes]] = subprocess.Popen,
    on_acp_event: Callable[[dict[str, Any]], None] | None = None,
) -> dict[str, Any]:
    """Run a single Grok ACP session via ``grok agent stdio``.

    Launches Grok in ACP stdio mode, negotiates the JSON-RPC 2.0 lifecycle
    (initialize → session/new → session/prompt), captures all agent→client
    events as metadata, and returns a receipt.

    When *on_acp_event* is provided it is called with metadata-only event
    dicts in real time as ACP notifications arrive — enabling live TUI
    streaming without waiting for session completion.

    This is the primary observability path per
    :file:`docs/GROK_CLI_SPECIALIZATION_ADAPTATION_AUDIT_2026-07-30.md` §7.

    Delegates to :class:`GrokAcpSession` — use that class directly for
    multi-prompt sessions.
    """
    with GrokAcpSession(request, config, popen_factory=popen_factory) as session:
        session.send_prompt(request.prompt_text, on_acp_event=on_acp_event)
    return session.receipt if session.receipt is not None else {}


# [D1.10 cleanup: old run_grok_acp_once body removed — replaced by GrokAcpSession wrapper above]
def run_grok_version_smoke(
    *,
    run_root: Path,
    workspace_path: Path | None = None,
    run_id: str | None = None,
    retrieval_mode: str = DEFAULT_RETRIEVAL_MODE,
    retrieval_mode_explicit: bool = False,
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
            retrieval_mode=retrieval_mode,
            retrieval_mode_explicit=retrieval_mode_explicit,
        ),
        config=config,
    )
