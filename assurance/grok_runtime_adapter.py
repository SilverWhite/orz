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
from .job_object_supervisor import (
    CREATE_SUSPENDED,
    JobObjectSupervisor,
    _resume_main_thread,
    contained_run,
)
from .utils import atomic_write_json, load_json, sha256_file, utc_now


ROOT = Path(__file__).resolve().parents[1]
ADAPTER_ID = "grok-runtime-adapter"
ADAPTER_VERSION = "0.1.0"
RECEIPT_SCHEMA = "grok-runtime-receipt-v0.1.schema.json"
CREATE_NO_WINDOW = getattr(subprocess, "CREATE_NO_WINDOW", 0)
SUPPORTED_RETRIEVAL_MODES = ("local_browser", "framework_fallback", "off")
DEFAULT_RETRIEVAL_MODE = "off"


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
    max_turns: int = 1


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
    """
    cfg = config or GrokRuntimeConfig()
    if request.mode != "acp_smoke":
        raise AssuranceError(f"run_grok_acp_once requires mode 'acp_smoke', got {request.mode}")
    if not request.prompt_text:
        raise AssuranceError("prompt_text is required for acp_smoke")
    retrieval_mode = validate_grok_retrieval_mode(request.retrieval_mode)
    _ensure_empty_run_root(request.run_root)
    if not request.workspace_path.is_dir():
        request.workspace_path.mkdir(parents=True, exist_ok=True)

    created_at = utc_now()
    trust_path = request.run_root / "workspace-trust.json"
    inspection_path = request.run_root / "binary-inspection.json"
    receipt_path = request.run_root / "grok-runtime-receipt.json"
    events_path = request.run_root / "events.jsonl"
    acp_transcript_path = request.run_root / "acp_transcript.jsonl"
    stderr_path = request.run_root / "grok.stderr.log"
    profile = request.run_root / "profile"
    temp = request.run_root / "temp"
    profile.mkdir()
    temp.mkdir()

    # Pre-flight: binary inspection + workspace trust (same as headless).
    inspection = inspect_grok_runtime(cfg)
    atomic_write_json(inspection_path, inspection)
    if inspection.get("valid") is not True:
        raise AssuranceError("locked Grok binary inspection is invalid")

    trust = _workspace_trust(config=cfg, request=request, output_path=trust_path)
    trust_launch_permitted = _trust_launch_permitted(trust)
    if trust.get("valid") is not True or not trust_launch_permitted:
        raise AssuranceError("workspace trust was not granted before Grok launch")

    binary_path = Path(str(inspection["binary_path"]))

    # Containment (same pattern as headless).
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

    # Launch grok agent stdio — stdin/stdout are PIPEs for JSON-RPC,
    # stderr goes to a file (same as headless paths).
    stderr_handle = stderr_path.open("wb")
    process = popen_factory(
        [str(binary_path), "agent", "--model", request.model_id, "stdio"],
        cwd=request.workspace_path,
        env=_clean_environment(profile, temp),
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=stderr_handle,
        text=True,
        encoding="utf-8",
        errors="replace",
        bufsize=1,
        creationflags=CREATE_NO_WINDOW | CREATE_SUSPENDED,
    )
    try:
        if supervisor is not None and supervisor.is_active:
            supervisor.assign_process(process.pid)
            containment_diag["job_object_assigned"] = supervisor.is_assigned
            _resume_main_thread(process.pid)

        # ── ACP lifecycle ──────────────────────────────────────────────
        acp_events: list[dict[str, Any]] = []
        session_id = ""
        protocol_version = 0
        tool_call_count = 0
        permission_requests_count = 0
        permission_outcomes: list[str] = []
        turn_count = 0
        stop_reason = ""

        def _acp_send(msg: dict[str, Any]) -> None:
            if process.stdin is None or process.stdin.closed:
                raise AssuranceError("Grok ACP stdin closed unexpectedly")
            line = json.dumps(msg, ensure_ascii=False, separators=(",", ":"), allow_nan=False)
            process.stdin.write(line + "\n")
            process.stdin.flush()
            acp_events.append({"direction": "client_to_agent", "message": msg})

        def _acp_receive(timeout: float = 30.0) -> dict[str, Any]:
            import select, time as _time
            deadline = _time.monotonic() + timeout
            while _time.monotonic() < deadline:
                if process.poll() is not None:
                    raise AssuranceError(
                        f"Grok ACP process exited early (code {process.returncode})"
                    )
                remaining = deadline - _time.monotonic()
                if remaining <= 0:
                    raise AssuranceError("timed out waiting for Grok ACP response")
                assert process.stdout is not None
                line = process.stdout.readline()
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
                acp_events.append({"direction": "agent_to_client", "message": msg})
                return msg
            raise AssuranceError("timed out waiting for Grok ACP response")

        def _acp_request(req_id: str, method: str, params: dict[str, Any]) -> dict[str, Any]:
            _acp_send({"jsonrpc": "2.0", "id": req_id, "method": method, "params": params})
            while True:
                msg = _acp_receive()
                if str(msg.get("id")) == req_id and "method" not in msg:
                    if "error" in msg:
                        raise AssuranceError(
                            f"ACP {method} error: {msg['error']}"
                        )
                    result = msg.get("result")
                    if not isinstance(result, dict):
                        raise AssuranceError(f"ACP {method} returned non-object result")
                    return result
                # Notifications → record and continue waiting.
                _handle_acp_notification(msg)

        def _handle_acp_notification(msg: dict[str, Any]) -> None:
            nonlocal tool_call_count, permission_requests_count, turn_count, stop_reason, session_id
            method = msg.get("method", "")
            params = msg.get("params", {}) if isinstance(msg.get("params"), dict) else {}
            if method == "session/update":
                update = params.get("update", {}) if isinstance(params.get("update"), dict) else {}
                update_type = update.get("sessionUpdate", "")
                if update_type == "tool_call":
                    tool_call_count += 1
                    if on_acp_event:
                        tool_info = update.get("tool_call", {}) if isinstance(update.get("tool_call"), dict) else {}
                        on_acp_event({
                            "event_type": "tool_proposal",
                            "timestamp": utc_now(),
                            "payload": {
                                "tool_name": tool_info.get("title", tool_info.get("name", "")),
                                "tool_call_id": tool_info.get("id", ""),
                            },
                            "redaction": "metadata_only",
                        })
                elif update_type == "turn_completed":
                    turn_count += 1
                    stop_reason = update.get("stop_reason", stop_reason)
            elif method == "session/request_permission" and "id" in msg:
                permission_requests_count += 1
                options = params.get("options", [])
                option_labels = [
                    o.get("kind", "") for o in options
                    if isinstance(o, dict)
                ]
                if on_acp_event:
                    tool_name = ""
                    if options:
                        first = options[0] if isinstance(options[0], dict) else {}
                        tool_name = first.get("toolTitle", first.get("title", ""))
                    user_decision = on_acp_event({
                        "event_type": "permission_requested",
                        "timestamp": utc_now(),
                        "payload": {
                            "permission": tool_name,
                            "options": option_labels,
                        },
                        "redaction": "metadata_only",
                    })
                else:
                    tool_name = ""
                    user_decision = None
                allow_id = next(
                    (o["optionId"] for o in options
                     if isinstance(o, dict) and o.get("kind") == "allow_once"),
                    None,
                )
                # Honour user decision when provided (interactive mode).
                # Fall back to auto-decision (allow_once if available, else cancel).
                if user_decision == "allow_once" and allow_id is not None:
                    _acp_send({
                        "jsonrpc": "2.0", "id": msg["id"],
                        "result": {"outcome": {"outcome": "selected", "optionId": allow_id}},
                    })
                    permission_outcomes.append("allow_once")
                elif user_decision == "allow_once" and allow_id is None:
                    # User wants to allow but no allow_once option exists — cancel.
                    _acp_send({
                        "jsonrpc": "2.0", "id": msg["id"],
                        "result": {"outcome": {"outcome": "cancelled"}},
                    })
                    permission_outcomes.append("cancelled")
                elif isinstance(user_decision, str) and user_decision:
                    # User explicitly denied or chose another option.
                    _acp_send({
                        "jsonrpc": "2.0", "id": msg["id"],
                        "result": {"outcome": {"outcome": "cancelled"}},
                    })
                    permission_outcomes.append(user_decision)
                elif allow_id is not None:
                    # Auto mode: default to allow_once when available.
                    _acp_send({
                        "jsonrpc": "2.0", "id": msg["id"],
                        "result": {"outcome": {"outcome": "selected", "optionId": allow_id}},
                    })
                    permission_outcomes.append("allow_once")
                else:
                    # Auto mode: no allow_once available — must cancel.
                    _acp_send({
                        "jsonrpc": "2.0", "id": msg["id"],
                        "result": {"outcome": {"outcome": "cancelled"}},
                    })
                    permission_outcomes.append("cancelled")
                decision_source = "user" if user_decision else "adapter"
                if on_acp_event:
                    on_acp_event({
                        "event_type": "permission_decision",
                        "timestamp": utc_now(),
                        "payload": {
                            "permission": tool_name if tool_name else option_labels[0] if option_labels else "",
                            "decision": permission_outcomes[-1] if permission_outcomes else "cancelled",
                            "decision_source": decision_source,
                        },
                        "redaction": "metadata_only",
                    })
            elif method == "_x.ai/session/prompt_complete":
                stop_reason = params.get("stopReason", stop_reason)
                if params.get("sessionId"):
                    session_id = params["sessionId"]

        # ACP lifecycle.
        init_result = _acp_request("acp-init-1", "initialize", {
            "protocolVersion": 1,
            "clientCapabilities": {},
            "clientInfo": {"name": "gsa-acp-adapter", "title": "GSA ACP Adapter", "version": "0.1.0"},
        })
        protocol_version = init_result.get("protocolVersion", 0)
        if on_acp_event:
            on_acp_event({
                "event_type": "acp_initialize",
                "timestamp": utc_now(),
                "payload": {"protocol_version": protocol_version},
                "redaction": "metadata_only",
            })

        session_result = _acp_request("acp-session-1", "session/new", {
            "cwd": str(request.workspace_path.resolve()),
            "mcpServers": [],
        })
        session_id = session_result.get("sessionId", session_id)
        if on_acp_event:
            on_acp_event({
                "event_type": "acp_session_created",
                "timestamp": utc_now(),
                "payload": {
                    "session_id_hash": sha256_bytes(
                        session_id.encode("utf-8")
                    ) if session_id else "",
                },
                "redaction": "metadata_only",
            })

        _acp_send({
            "jsonrpc": "2.0", "id": "acp-prompt-1",
            "method": "session/prompt",
            "params": {
                "sessionId": session_id,
                "prompt": [{"type": "text", "text": request.prompt_text}],
            },
        })
        prompt_result: dict[str, Any] = {}
        while True:
            msg = _acp_receive(timeout=cfg.timeout_seconds)
            if str(msg.get("id")) == "acp-prompt-1" and "method" not in msg:
                if "error" in msg:
                    raise AssuranceError(f"session/prompt error: {msg['error']}")
                prompt_result = msg.get("result", {}) if isinstance(msg.get("result"), dict) else {}
                break
            _handle_acp_notification(msg)

        # Drain remaining notifications after prompt completes.
        import time as _time
        drain_deadline = _time.monotonic() + 5.0
        while _time.monotonic() < drain_deadline and process.poll() is None:
            assert process.stdout is not None
            import select as _sel
            ready, _, _ = _sel.select([process.stdout], [], [], 0.5)
            if not ready:
                break
            line = process.stdout.readline()
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
                _handle_acp_notification(msg)
                acp_events.append({"direction": "agent_to_client", "message": msg})

        # Close stdin and wait for exit.
        if process.stdin is not None and not process.stdin.closed:
            process.stdin.close()
        try:
            process.wait(timeout=10)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait(timeout=5)

        # Write ACP transcript to disk (metadata: direction + message digest only).
        import json as _json
        transcript_lines = [
            _json.dumps(
                {"direction": e["direction"],
                 "message_sha256": sha256_bytes(
                     _json.dumps(e["message"], ensure_ascii=False, sort_keys=True,
                                 separators=(",", ":"), allow_nan=False).encode("utf-8")
                 )},
                ensure_ascii=False, sort_keys=True, separators=(",", ":"), allow_nan=False,
            )
            for e in acp_events
        ]
        acp_transcript_path.write_text(
            "\n".join(transcript_lines) + "\n", encoding="utf-8",
        )

    finally:
        stderr_handle.close()
        if supervisor is not None:
            supervisor.close()

    root_process_exited = process.poll() is not None
    checks = {
        "binary_inspection_valid": inspection.get("valid") is True,
        "workspace_trust_valid": trust.get("valid") is True,
        "workspace_trust_granted": trust_launch_permitted,
        "root_process_exited": root_process_exited,
        "no_residue_observed": root_process_exited,
        "exit_code_zero": process.returncode == 0,
        "acp_initialize_ok": protocol_version >= 1,
        "acp_session_created": bool(session_id),
        "acp_prompt_completed": bool(prompt_result),
        "adapter_containment_available": containment_diag["containment_available"],
        "adapter_containment_provided": containment_diag["job_object_assigned"],
    }
    prompt_sha256_val = sha256_bytes((request.prompt_text or "").encode("utf-8"))
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
            "active_for_current_mode": retrieval_mode != "off",
            "runtime_tool_retrieval_allowed": retrieval_mode != "off",
            "assurance_receipts_required": retrieval_mode != "off",
            "valid_modes": list(SUPPORTED_RETRIEVAL_MODES),
        },
        "binary": {
            "inspection_path": str(inspection_path.resolve()),
            "binary_path": str(binary_path.resolve()),
            "version_output": "",
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
            "command_kind": "grok_acp",
            "pid": int(process.pid),
            "exit_code": process.returncode,
            "timeout_seconds": cfg.timeout_seconds,
            "stdout_path": str(acp_transcript_path.resolve()),
            "stderr_path": str(stderr_path.resolve()),
            "stdout_sha256": sha256_file(acp_transcript_path) if acp_transcript_path.exists() else "",
            "stderr_sha256": sha256_file(stderr_path),
        },
        "acp": {
            "protocol_version": protocol_version,
            "session_id_hash": sha256_bytes(session_id.encode("utf-8")) if session_id else "",
            "tool_call_count": tool_call_count,
            "permission_requests_count": permission_requests_count,
            "permission_outcomes": permission_outcomes,
            "turn_count": turn_count,
            "stop_reason": stop_reason,
            "event_count": len(acp_events),
        },
        "prompt": {
            "prompt_sha256": prompt_sha256_val,
            "prompt_bytes": len(request.prompt_text or ""),
            "model_id": request.model_id,
            "max_turns": request.max_turns,
            "response_summary": "",
            "response_sha256": "",
            "response_finish_reason": stop_reason,
            "response_token_count": 0,
            "output_format": "acp_jsonrpc",
        },
        "containment": {
            "no_residue_required": True,
            "no_residue_observed": root_process_exited,
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
        "limitations": [
            "ACP smoke: first ACP session — observe agent lifecycle, permission handling, tool calls.",
            "Adapter-side containment via CREATE_SUSPENDED + JobObjectSupervisor.",
            "ACP transcript captured as metadata (hashes, counts); raw prompt/responses are not stored.",
            "Session files on disk (if any) are not read post-run; only in-session ACP events are recorded.",
        ],
    }
    validate_contract(receipt, RECEIPT_SCHEMA, label="Grok runtime receipt")
    atomic_write_json(receipt_path, receipt)
    events = normalize_grok_runtime_receipt(receipt, created_at=created_at)
    write_grok_events_jsonl(events_path, events)

    # ── D1.12: Post-run cross-verification ──────────────────────────────
    try:
        from .grok_session_verifier import verify_acp_session, write_verification_receipt
        verification = verify_acp_session(
            receipt,
            events_path=events_path,
            transcript_path=acp_transcript_path,
        )
        verification_path = request.run_root / "session-verification.json"
        write_verification_receipt(verification, verification_path)
        receipt["artifacts"]["verification_path"] = str(verification_path.resolve())
        receipt["artifacts"]["verification_valid"] = verification["valid"]
    except Exception:
        # Verification is a non-fatal assurance check.
        receipt["artifacts"]["verification_path"] = ""
        receipt["artifacts"]["verification_valid"] = False

    return receipt


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
