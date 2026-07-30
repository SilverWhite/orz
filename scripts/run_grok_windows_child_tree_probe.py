from __future__ import annotations

import argparse
import ctypes
from ctypes import wintypes
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import time
import uuid
from typing import Any


SCENARIO_TO_PROVIDER = {
    "tool_timeout": "process-timeout",
    "task_cancel": "process-cancel",
    "parent_exit": "process-parent-exit",
}
ROLES = ("root", "child", "grandchild")
CREATE_NO_WINDOW = getattr(subprocess, "CREATE_NO_WINDOW", 0)


def _atomic_write_json(path: Path, value: Any) -> None:
    temporary = path.with_name(f".{path.name}.{os.getpid()}.{time.time_ns()}.tmp")
    temporary.write_text(
        json.dumps(value, ensure_ascii=False, indent=2, sort_keys=True, allow_nan=False)
        + "\n",
        encoding="utf-8",
    )
    temporary.replace(path)


def _sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def _artifact(path: Path) -> dict[str, Any]:
    content = path.read_bytes()
    return {
        "path": str(path.resolve()),
        "bytes": len(content),
        "sha256": _sha256_bytes(content),
    }


def _read_json(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ValueError(f"expected a JSON object: {path}")
    return value


def _write_utf8(path: Path, value: str) -> None:
    temporary = path.with_name(f".{path.name}.{os.getpid()}.{time.time_ns()}.tmp")
    temporary.write_text(value, encoding="utf-8", newline="\n")
    temporary.replace(path)


def _require_windows_admin() -> None:
    if os.name != "nt":
        raise RuntimeError("the Grok child-tree probe is Windows-only")
    if not ctypes.windll.shell32.IsUserAnAdmin():
        raise PermissionError(
            "Administrator token required: temporary firewall rules are fail-closed"
        )


class KillOnCloseJob:
    JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE = 0x00002000
    JOB_OBJECT_EXTENDED_LIMIT_INFORMATION = 9

    class IO_COUNTERS(ctypes.Structure):
        _fields_ = [
            ("ReadOperationCount", ctypes.c_ulonglong),
            ("WriteOperationCount", ctypes.c_ulonglong),
            ("OtherOperationCount", ctypes.c_ulonglong),
            ("ReadTransferCount", ctypes.c_ulonglong),
            ("WriteTransferCount", ctypes.c_ulonglong),
            ("OtherTransferCount", ctypes.c_ulonglong),
        ]

    class JOBOBJECT_BASIC_LIMIT_INFORMATION(ctypes.Structure):
        _fields_ = [
            ("PerProcessUserTimeLimit", ctypes.c_longlong),
            ("PerJobUserTimeLimit", ctypes.c_longlong),
            ("LimitFlags", wintypes.DWORD),
            ("MinimumWorkingSetSize", ctypes.c_size_t),
            ("MaximumWorkingSetSize", ctypes.c_size_t),
            ("ActiveProcessLimit", wintypes.DWORD),
            ("Affinity", ctypes.c_size_t),
            ("PriorityClass", wintypes.DWORD),
            ("SchedulingClass", wintypes.DWORD),
        ]

    class JOBOBJECT_EXTENDED_LIMIT_INFORMATION(ctypes.Structure):
        pass

    JOBOBJECT_EXTENDED_LIMIT_INFORMATION._fields_ = [
        ("BasicLimitInformation", JOBOBJECT_BASIC_LIMIT_INFORMATION),
        ("IoInfo", IO_COUNTERS),
        ("ProcessMemoryLimit", ctypes.c_size_t),
        ("JobMemoryLimit", ctypes.c_size_t),
        ("PeakProcessMemoryUsed", ctypes.c_size_t),
        ("PeakJobMemoryUsed", ctypes.c_size_t),
    ]

    def __init__(self) -> None:
        kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
        self._close_handle = kernel32.CloseHandle
        self._close_handle.argtypes = [wintypes.HANDLE]
        self._close_handle.restype = wintypes.BOOL
        create = kernel32.CreateJobObjectW
        create.argtypes = [ctypes.c_void_p, wintypes.LPCWSTR]
        create.restype = wintypes.HANDLE
        set_information = kernel32.SetInformationJobObject
        set_information.argtypes = [
            wintypes.HANDLE,
            ctypes.c_int,
            ctypes.c_void_p,
            wintypes.DWORD,
        ]
        set_information.restype = wintypes.BOOL
        self._assign = kernel32.AssignProcessToJobObject
        self._assign.argtypes = [wintypes.HANDLE, wintypes.HANDLE]
        self._assign.restype = wintypes.BOOL
        self.handle = create(None, None)
        if not self.handle:
            raise ctypes.WinError(ctypes.get_last_error())
        self.closed = False
        self.assigned = False
        information = self.JOBOBJECT_EXTENDED_LIMIT_INFORMATION()
        information.BasicLimitInformation.LimitFlags = (
            self.JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
        )
        if not set_information(
            self.handle,
            self.JOB_OBJECT_EXTENDED_LIMIT_INFORMATION,
            ctypes.byref(information),
            ctypes.sizeof(information),
        ):
            error = ctypes.get_last_error()
            self.close()
            raise ctypes.WinError(error)

    def assign(self, process: subprocess.Popen[bytes]) -> None:
        process_handle = wintypes.HANDLE(int(process._handle))  # type: ignore[attr-defined]
        if not self._assign(self.handle, process_handle):
            raise ctypes.WinError(ctypes.get_last_error())
        self.assigned = True

    def close(self) -> None:
        if getattr(self, "closed", True):
            return
        if self.handle and not self._close_handle(self.handle):
            raise ctypes.WinError(ctypes.get_last_error())
        self.closed = True
        self.handle = None


def _run_json(command: list[str], *, env: dict[str, str] | None = None) -> dict[str, Any]:
    completed = subprocess.run(
        command,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        encoding="utf-8",
        errors="replace",
        env=env,
        timeout=60,
        check=False,
        creationflags=CREATE_NO_WINDOW,
    )
    if completed.returncode != 0:
        raise RuntimeError(
            f"command failed ({completed.returncode}): {completed.stderr.strip()}"
        )
    value = json.loads(completed.stdout)
    if not isinstance(value, dict):
        raise RuntimeError("JSON command did not return an object")
    return value


def _trust_receipt(repo_root: Path, workspace: Path, output: Path) -> dict[str, Any]:
    return _run_json(
        [
            "powershell",
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            str(repo_root / "scripts" / "new_grok_workspace_trust_receipt.ps1"),
            "-WorkspacePath",
            str(workspace),
            "-OutputPath",
            str(output),
            "-Decision",
            "restricted",
        ]
    )


def _firewall(
    repo_root: Path,
    action: str,
    prefix: str,
    programs: list[Path] | None = None,
) -> dict[str, Any]:
    command = [
        "powershell",
        "-NoProfile",
        "-ExecutionPolicy",
        "Bypass",
        "-File",
        str(repo_root / "scripts" / "manage_grok_probe_firewall.ps1"),
        "-Action",
        action,
        "-RulePrefix",
        prefix,
    ]
    if programs:
        command.extend(
            [
                "-ProgramListJson",
                json.dumps([str(path) for path in programs], separators=(",", ":")),
            ]
        )
    return _run_json(command)


def _scan_nonce_processes(nonce: str) -> list[dict[str, Any]]:
    script = (
        "$n=$env:LIF_CHILD_TREE_NONCE;"
        "@(Get-CimInstance Win32_Process | "
        "Where-Object { $_.CommandLine -and $_.CommandLine.Contains($n) } | "
        "Select-Object ProcessId,ParentProcessId,Name,CreationDate,CommandLine) | "
        "ConvertTo-Json -Depth 5 -Compress"
    )
    environment = dict(os.environ)
    environment["LIF_CHILD_TREE_NONCE"] = nonce
    completed = subprocess.run(
        ["powershell", "-NoProfile", "-Command", script],
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        encoding="utf-8",
        errors="replace",
        env=environment,
        timeout=20,
        check=False,
        creationflags=CREATE_NO_WINDOW,
    )
    if completed.returncode != 0:
        raise RuntimeError(f"process scan failed: {completed.stderr.strip()}")
    raw = completed.stdout.strip()
    if not raw:
        return []
    parsed = json.loads(raw)
    rows = parsed if isinstance(parsed, list) else [parsed]
    projected: list[dict[str, Any]] = []
    for row in rows:
        if not isinstance(row, dict):
            continue
        command_line = str(row.get("CommandLine") or "")
        projected.append(
            {
                "pid": int(row["ProcessId"]),
                "parent_pid": int(row["ParentProcessId"]),
                "name": str(row.get("Name") or ""),
                "creation_date": str(row.get("CreationDate") or ""),
                "command_line_bytes": len(command_line.encode("utf-8")),
                "command_line_sha256": _sha256_bytes(command_line.encode("utf-8")),
                "nonce_present": nonce in command_line,
                "command_line_recorded": False,
            }
        )
    return sorted(projected, key=lambda row: row["pid"])


def _wait_for_files(
    paths: list[Path],
    deadline: float,
    *,
    process: subprocess.Popen[bytes] | None = None,
) -> None:
    while time.monotonic() < deadline:
        if all(path.is_file() for path in paths):
            return
        if process is not None and process.poll() is not None:
            raise RuntimeError(
                f"Grok exited before process-tree readiness with code {process.returncode}"
            )
        time.sleep(0.025)
    missing = [str(path) for path in paths if not path.is_file()]
    raise TimeoutError(f"timed out waiting for process records: {missing}")


def _read_jsonl(path: Path) -> list[dict[str, Any]]:
    if not path.is_file():
        return []
    rows: list[dict[str, Any]] = []
    for line in path.read_text(encoding="utf-8").splitlines():
        if not line.strip():
            continue
        value = json.loads(line)
        if isinstance(value, dict):
            rows.append(value)
    return rows


def _primary_requests(path: Path) -> list[dict[str, Any]]:
    return [
        row
        for row in _read_jsonl(path)
        if isinstance(row.get("body"), dict)
        and row["body"].get("model") == "deepseek-v4-pro"
    ]


def _wait_for_primary_requests(path: Path, count: int, deadline: float) -> None:
    while time.monotonic() < deadline:
        if len(_primary_requests(path)) >= count:
            return
        time.sleep(0.025)
    raise TimeoutError(f"timed out waiting for {count} primary provider requests")


def _tool_sequence(requests: list[dict[str, Any]]) -> list[str]:
    sequence: list[str] = []
    for request in requests:
        body = request.get("body")
        messages = body.get("messages", []) if isinstance(body, dict) else []
        for message in messages:
            if not isinstance(message, dict) or message.get("role") != "assistant":
                continue
            calls = message.get("tool_calls", [])
            if not isinstance(calls, list):
                continue
            for call in calls:
                function = call.get("function") if isinstance(call, dict) else None
                name = function.get("name") if isinstance(function, dict) else None
                if isinstance(name, str) and name not in sequence:
                    sequence.append(name)
    return sequence


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
            "LIF_FAKE_DEEPSEEK_KEY": "loopback-fixture-not-a-secret",
            "GROK_MEMORY": "0",
            "GROK_WEB_FETCH": "0",
            "HTTP_PROXY": "http://127.0.0.1:1",
            "HTTPS_PROXY": "http://127.0.0.1:1",
            "ALL_PROXY": "http://127.0.0.1:1",
            "NO_PROXY": "127.0.0.1,localhost",
        }
    )
    return environment


def _powershell_quote(value: str) -> str:
    if "'" in value or "\r" in value or "\n" in value:
        raise ValueError("fixture paths may not contain apostrophes or newlines")
    return f"'{value}'"


def _inspect_binary(
    repo_root: Path, binary_path: Path | None, release_metadata: Path
) -> dict[str, Any]:
    command = [
        "powershell",
        "-NoProfile",
        "-ExecutionPolicy",
        "Bypass",
        "-File",
        str(repo_root / "scripts" / "inspect_grok_install.ps1"),
        "-ReleaseMetadataPath",
        str(release_metadata),
    ]
    if binary_path is not None:
        command.extend(["-BinaryPath", str(binary_path)])
    inspection = _run_json(command)
    if inspection.get("valid") is not True:
        raise RuntimeError("Grok binary inspection did not validate against release metadata")
    return inspection


def run(args: argparse.Namespace) -> dict[str, Any]:
    _require_windows_admin()
    repo_root = Path(__file__).resolve().parents[1]
    output = args.output_directory.resolve()
    if output.exists():
        raise ValueError(f"refusing to overwrite output directory: {output}")
    release_metadata = args.release_metadata.resolve()
    release = _read_json(release_metadata)
    inspection = _inspect_binary(
        repo_root,
        args.binary.resolve() if args.binary else None,
        release_metadata,
    )
    binary = Path(str(inspection["binary_path"])).resolve()
    python_path = Path(args.python_path or sys.executable).resolve()
    fixture_script = (repo_root / "scripts" / "child_tree_fixture.py").resolve()
    provider_script = (repo_root / "scripts" / "fake_deepseek_provider.py").resolve()
    verify_script = (
        repo_root / "scripts" / "verify_grok_windows_child_tree_probe.py"
    ).resolve()

    started_at = time.time()
    probe_id = f"CHILDTREE-{uuid.uuid4().hex}"
    nonce = f"LIFCHILD-{uuid.uuid4().hex}"
    firewall_prefix = f"LIFGrokChild-{probe_id[10:26]}"
    scenario = args.scenario
    provider_scenario = SCENARIO_TO_PROVIDER[scenario]
    output.mkdir(parents=True)
    workspace = output / "workspace"
    process_records = workspace / "process-records"
    profile = output / "profile"
    config_directory = profile / ".grok"
    temp = output / "temp"
    provider_directory = output / "provider"
    workspace.mkdir()
    process_records.mkdir()
    config_directory.mkdir(parents=True)
    temp.mkdir()
    prompt_path = workspace / "prompt.txt"
    command_path = workspace / "terminal-command.txt"
    marker_path = workspace / ".lif-disposable-workspace.json"
    result_path = output / "result.json"
    verification_path = output / "verification.json"
    failure_path = output / "failure.json"
    firewall_receipt_path = output / "firewall-receipt.json"
    preflight_path = output / "workspace-trust.preflight.json"
    launch_path = output / "workspace-trust.launch.json"
    postrun_path = output / "workspace-trust.postrun.json"
    provider_stdout_path = output / "provider.stdout.log"
    provider_stderr_path = output / "provider.stderr.log"
    grok_stdout_path = output / "grok.stdout.log"
    grok_stderr_path = output / "grok.stderr.log"
    timeout_diagnostic_path = output / "timeout-diagnostic.json"
    private_capture_path = provider_directory / "requests.private.jsonl"
    role_paths = [process_records / f"{role}.json" for role in ROLES]

    _write_utf8(
        prompt_path,
        "Run only the exact fixed fake containment tool call supplied by the loopback "
        "provider. Do not infer or perform any other action.\n",
    )
    _atomic_write_json(
        marker_path,
        {
            "schema_version": "0.1.0",
            "marker_kind": "lif-disposable-workspace",
            "fixture_id": f"FIXTURE-{uuid.uuid4().hex}",
            "disposable": True,
        },
    )
    command = " ".join(
        [
            "&",
            _powershell_quote(str(python_path)),
            _powershell_quote(str(fixture_script)),
            "--output-directory",
            _powershell_quote(str(process_records)),
            "--nonce",
            _powershell_quote(nonce),
            "--hold-seconds",
            "300",
        ]
    )
    _write_utf8(command_path, command + "\n")
    preflight = _trust_receipt(repo_root, workspace, preflight_path)

    provider: subprocess.Popen[bytes] | None = None
    grok: subprocess.Popen[bytes] | None = None
    job: KillOnCloseJob | None = None
    firewall_added = False
    firewall_receipt: dict[str, Any] = {
        "schema_version": "0.1.0",
        "added": False,
        "removed": False,
        "remaining_rule_count": -1,
    }
    parent_exit_triggered = False
    pre_trigger_processes: list[dict[str, Any]] = []
    post_trigger_processes: list[dict[str, Any]] = []
    provider_stdout_handle = provider_stderr_handle = None
    grok_stdout_handle = grok_stderr_handle = None
    try:
        provider_stdout_handle = provider_stdout_path.open("wb")
        provider_stderr_handle = provider_stderr_path.open("wb")
        provider = subprocess.Popen(
            [
                str(python_path),
                str(provider_script),
                "--output-directory",
                str(provider_directory),
                "--timeout-seconds",
                str(args.timeout_seconds + 20),
                "--scenario",
                provider_scenario,
                "--terminal-command-path",
                str(command_path),
                "--process-record-directory",
                str(process_records),
                "--tool-timeout-ms",
                str(args.tool_timeout_ms),
            ],
            cwd=output,
            stdin=subprocess.DEVNULL,
            stdout=provider_stdout_handle,
            stderr=provider_stderr_handle,
            creationflags=CREATE_NO_WINDOW,
        )
        ready_path = provider_directory / "ready.json"
        deadline = time.monotonic() + 10
        while not ready_path.is_file():
            if provider.poll() is not None:
                raise RuntimeError("fake provider exited before readiness")
            if time.monotonic() >= deadline:
                raise TimeoutError("timed out waiting for fake provider readiness")
            time.sleep(0.05)
        ready = _read_json(ready_path)
        if (
            ready.get("ready") is not True
            or ready.get("host") != "127.0.0.1"
            or ready.get("external_bind") is not False
        ):
            raise RuntimeError("fake provider did not prove a loopback-only bind")

        config = f"""[cli]
auto_update = false
use_leader = false

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
base_url = "http://127.0.0.1:{ready['port']}"
name = "LIF Fake DeepSeek Loopback"
env_key = "LIF_FAKE_DEEPSEEK_KEY"
api_backend = "chat_completions"
max_completion_tokens = 256
context_window = 4096

[permission]
rules = [
  {{ action = "allow", tool = "bash" }},
  # Grok classifies task lifecycle operations as read-like permission
  # accesses.  Do not deny that class here: Read itself is absent from the
  # narrow --tools allowlist, while kill/get-output must remain executable.
  {{ action = "deny", tool = "edit" }},
  {{ action = "deny", tool = "grep" }},
  {{ action = "deny", tool = "mcp" }},
  {{ action = "deny", tool = "webfetch" }},
  {{ action = "deny", tool = "websearch" }},
]
"""
        _write_utf8(config_directory / "config.toml", config)
        add_receipt = _firewall(
            repo_root, "add", firewall_prefix, [binary, python_path]
        )
        firewall_added = True
        firewall_receipt.update(
            {
                "added": True,
                "add_receipt": add_receipt,
                "programs": [str(binary), str(python_path)],
            }
        )

        launch = _trust_receipt(repo_root, workspace, launch_path)
        if (
            launch["discovery"]["aggregate_sha256"]
            != preflight["discovery"]["aggregate_sha256"]
            or launch["discovery"]["scan_policy_sha256"]
            != preflight["discovery"]["scan_policy_sha256"]
        ):
            raise RuntimeError("workspace control surface changed before launch")

        grok_stdout_handle = grok_stdout_path.open("wb")
        grok_stderr_handle = grok_stderr_path.open("wb")
        max_turns = {"tool_timeout": "2", "task_cancel": "4", "parent_exit": "3"}[
            scenario
        ]
        grok = subprocess.Popen(
            [
                str(binary),
                "--cwd",
                str(workspace),
                "--model",
                "lif-fake-deepseek",
                "--reasoning-effort",
                "high",
                "--max-turns",
                max_turns,
                "--output-format",
                "streaming-json",
                "--session-id",
                str(uuid.uuid4()),
                "--no-memory",
                "--disable-web-search",
                "--permission-mode",
                "bypassPermissions",
                "--sandbox",
                "read-write",
                "--tools",
                # Grok 0.2.111 couples background Bash to its task-lifecycle
                # tools.  A Bash-only allowlist deliberately disables
                # enabled_background in AgentBuilder, so retain the lifecycle
                # dependencies while the deterministic provider still invokes
                # only the fixed Bash/cancel/output sequence.
                "Bash,Task(lif-probe-disabled)",
                "--verbatim",
                "--prompt-file",
                str(prompt_path),
            ],
            cwd=workspace,
            env=_clean_environment(profile, temp),
            stdin=subprocess.DEVNULL,
            stdout=grok_stdout_handle,
            stderr=grok_stderr_handle,
            creationflags=CREATE_NO_WINDOW,
        )
        job = KillOnCloseJob()
        job.assign(grok)

        _wait_for_files(
            role_paths,
            time.monotonic() + min(15, args.timeout_seconds),
            process=grok,
        )
        pre_trigger_processes = _scan_nonce_processes(nonce)
        if scenario == "task_cancel":
            _atomic_write_json(
                process_records / "driver-pre-trigger-ready.json",
                {
                    "probe_id": probe_id,
                    "nonce": nonce,
                    "process_count": len(pre_trigger_processes),
                    "recorded_at_unix_ns": time.time_ns(),
                },
            )
        if scenario == "parent_exit":
            _wait_for_primary_requests(
                private_capture_path, 2, time.monotonic() + 10
            )
            if grok.poll() is not None:
                raise RuntimeError("Grok exited before the parent-exit trigger")
            job.close()
            parent_exit_triggered = True
        try:
            grok.wait(timeout=args.timeout_seconds)
        except subprocess.TimeoutExpired as error:
            try:
                grok.wait(timeout=args.exit_grace_seconds)
            except subprocess.TimeoutExpired:
                pass
            else:
                error = None
            if grok.poll() is not None:
                error = None
            if error is None:
                pass
            else:
                for handle in (
                    grok_stdout_handle,
                    grok_stderr_handle,
                    provider_stdout_handle,
                    provider_stderr_handle,
                ):
                    if handle is not None and not handle.closed:
                        handle.flush()
                timeout_primary = _primary_requests(private_capture_path)
                timeout_combined_capture = b"".join(
                    path.read_bytes()
                    for path in (private_capture_path, grok_stdout_path, grok_stderr_path)
                    if path.is_file()
                )
                before_job_close_processes = _scan_nonce_processes(nonce)
                provider_result_path = provider_directory / "provider-result.json"
                provider_result_value = (
                    _read_json(provider_result_path)
                    if provider_result_path.is_file()
                    else None
                )
                job_closed_before_diagnostic = job is not None and job.closed
                if job is not None:
                    job.close()
                time.sleep(0.75)
                after_job_close_processes = _scan_nonce_processes(nonce)
                _atomic_write_json(
                    timeout_diagnostic_path,
                    {
                        "schema_version": "0.1.0",
                        "diagnostic_kind": "grok-windows-child-tree-timeout",
                        "probe_id": probe_id,
                        "scenario": scenario,
                        "nonce": nonce,
                        "elapsed_seconds": round(time.time() - started_at, 3),
                        "timeout_seconds": args.timeout_seconds,
                        "exit_grace_seconds": args.exit_grace_seconds,
                        "tool_timeout_ms": args.tool_timeout_ms,
                        "grok": {
                            "pid": grok.pid if grok is not None else None,
                            "returncode_before_job_close": (
                                grok.poll() if grok is not None else None
                            ),
                        },
                        "provider": {
                            "pid": provider.pid if provider is not None else None,
                            "returncode_before_job_close": (
                                provider.poll() if provider is not None else None
                            ),
                            "primary_request_count": len(timeout_primary),
                            "tool_sequence": _tool_sequence(timeout_primary),
                            "result_exists": provider_result_path.is_file(),
                            "result": provider_result_value,
                        },
                        "process_tree": {
                            "role_files_present": [
                                path.name for path in role_paths if path.is_file()
                            ],
                            "before_job_close_processes": before_job_close_processes,
                            "after_job_close_processes": after_job_close_processes,
                        },
                        "capture": {
                            "grok_stdout": (
                                _artifact(grok_stdout_path)
                                if grok_stdout_path.is_file()
                                else None
                            ),
                            "grok_stderr": (
                                _artifact(grok_stderr_path)
                                if grok_stderr_path.is_file()
                                else None
                            ),
                            "provider_stdout": (
                                _artifact(provider_stdout_path)
                                if provider_stdout_path.is_file()
                                else None
                            ),
                            "provider_stderr": (
                                _artifact(provider_stderr_path)
                                if provider_stderr_path.is_file()
                                else None
                            ),
                            "marker_projection": {
                                role: {
                                    "stdout": (
                                        f"LIF_CHILD_TREE_STDOUT:{nonce}:{role}:ready".encode()
                                        in timeout_combined_capture
                                    ),
                                    "stderr": (
                                        f"LIF_CHILD_TREE_STDERR:{nonce}:{role}:ready".encode()
                                        in timeout_combined_capture
                                    ),
                                }
                                for role in ROLES
                            },
                        },
                        "job": {
                            "created": job is not None,
                            "assigned": job is not None and job.assigned,
                            "closed_before_diagnostic": job_closed_before_diagnostic,
                            "closed_after_diagnostic": job is not None and job.closed,
                        },
                        "diagnostic_boundary": (
                            "Collected after Grok exceeded the outer expected-terminal-state "
                            "timeout and grace window, before raising the gate failure."
                        ),
                    },
                )
                raise TimeoutError(
                    "Grok did not reach the expected terminal state"
                ) from error
        if job is not None and not job.closed:
            job.close()
        time.sleep(0.75)
        post_trigger_processes = _scan_nonce_processes(nonce)
        try:
            provider.wait(timeout=10)
        except subprocess.TimeoutExpired:
            provider.terminate()
            provider.wait(timeout=5)
            raise TimeoutError("fake provider did not terminate after Grok")

        for handle in (
            grok_stdout_handle,
            grok_stderr_handle,
            provider_stdout_handle,
            provider_stderr_handle,
        ):
            if handle is not None and not handle.closed:
                handle.close()
        postrun = _trust_receipt(repo_root, workspace, postrun_path)
        if (
            postrun["discovery"]["aggregate_sha256"]
            != preflight["discovery"]["aggregate_sha256"]
            or postrun["discovery"]["scan_policy_sha256"]
            != preflight["discovery"]["scan_policy_sha256"]
        ):
            raise RuntimeError("workspace control surface changed during the probe")
    except Exception as error:
        _atomic_write_json(
            failure_path,
            {
                "schema_version": "0.1.0",
                "failure_kind": "grok-windows-child-tree-probe",
                "probe_id": probe_id,
                "scenario": scenario,
                "message": f"{type(error).__name__}: {error}",
                "failed_at_unix_ns": time.time_ns(),
            },
        )
        raise
    finally:
        if job is not None and not job.closed:
            try:
                job.close()
            except Exception:
                pass
        for process in (grok, provider):
            if process is not None and process.poll() is None:
                process.terminate()
                try:
                    process.wait(timeout=3)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait(timeout=3)
        for handle in (
            grok_stdout_handle,
            grok_stderr_handle,
            provider_stdout_handle,
            provider_stderr_handle,
        ):
            if handle is not None and not handle.closed:
                handle.close()
        if firewall_added:
            try:
                remove_receipt = _firewall(repo_root, "remove", firewall_prefix)
                firewall_receipt.update(
                    {
                        "removed": remove_receipt.get("removed") is True,
                        "remaining_rule_count": remove_receipt.get(
                            "remaining_rule_count", -1
                        ),
                        "remove_receipt": remove_receipt,
                    }
                )
            finally:
                _atomic_write_json(firewall_receipt_path, firewall_receipt)

    role_records = [_read_json(path) for path in role_paths]
    roles_by_name = {str(record.get("role")): record for record in role_records}
    relationship_valid = (
        set(roles_by_name) == set(ROLES)
        and roles_by_name["child"].get("parent_pid") == roles_by_name["root"].get("pid")
        and roles_by_name["grandchild"].get("parent_pid")
        == roles_by_name["child"].get("pid")
    )
    primary = _primary_requests(private_capture_path)
    tool_sequence = _tool_sequence(primary)
    provider_result_path = provider_directory / "provider-result.json"
    provider_result_value = _read_json(provider_result_path)
    expected_request_count = {
        "tool_timeout": 2,
        "task_cancel": 4,
        "parent_exit": 2,
    }[scenario]
    expected_tool_sequence = {
        "tool_timeout": ["run_terminal_command"],
        "task_cancel": [
            "run_terminal_command",
            "kill_command_or_subagent",
            "get_command_or_subagent_output",
        ],
        "parent_exit": ["run_terminal_command"],
    }[scenario]
    combined_capture = b"".join(
        path.read_bytes()
        for path in (private_capture_path, grok_stdout_path, grok_stderr_path)
        if path.is_file()
    )
    marker_projection = {
        role: {
            "stdout": f"LIF_CHILD_TREE_STDOUT:{nonce}:{role}:ready".encode()
            in combined_capture,
            "stderr": f"LIF_CHILD_TREE_STDERR:{nonce}:{role}:ready".encode()
            in combined_capture,
        }
        for role in ROLES
    }
    output_drain_required = scenario in {"tool_timeout", "task_cancel"}
    output_drain_observed = all(
        values["stdout"] and values["stderr"] for values in marker_projection.values()
    )
    checks = {
        "binary_matches_release": inspection.get("valid") is True,
        "outer_job_created": job is not None,
        "outer_job_assigned": job is not None and job.assigned,
        "outer_job_closed": job is not None and job.closed,
        "all_roles_observed": set(roles_by_name) == set(ROLES),
        "pid_relationships_valid": relationship_valid,
        "no_normal_completion": all(
            record.get("normal_completion") is False for record in role_records
        ),
        "pre_trigger_tree_live": all(
            int(record["pid"]) in {row["pid"] for row in pre_trigger_processes}
            for record in role_records
        ),
        "post_trigger_residue_zero": len(post_trigger_processes) == 0,
        "provider_request_count_matches": len(primary) == expected_request_count,
        "provider_tool_sequence_matches": tool_sequence == expected_tool_sequence,
        "provider_terminal_succeeded": provider_result_value.get("terminal_state")
        == "succeeded",
        "provider_parent_disconnect_matches": (
            provider_result_value.get("continuity", {}).get(
                "parent_exit_disconnect_observed"
            )
            is True
            if scenario == "parent_exit"
            else True
        ),
        "real_model_not_invoked": True,
        "output_drain_matches_policy": (
            output_drain_observed if output_drain_required else True
        ),
        "parent_exit_trigger_matches": (
            parent_exit_triggered if scenario == "parent_exit" else not parent_exit_triggered
        ),
        "workspace_control_unchanged": True,
        "firewall_removed": firewall_receipt.get("removed") is True
        and firewall_receipt.get("remaining_rule_count") == 0,
    }
    observed = inspection["observed"]
    binary_release = release["binary_release"]
    result = {
        "schema_version": "0.1.0",
        "probe_kind": "grok-windows-child-tree",
        "probe_id": probe_id,
        "scenario": scenario,
        "valid": all(checks.values()),
        "started_at_unix_ns": int(started_at * 1_000_000_000),
        "completed_at_unix_ns": time.time_ns(),
        "binary": {
            "path": str(binary),
            "version": str(observed["version_output"]),
            "bytes": int(observed["bytes"]),
            "sha256": str(observed["sha256"]),
            "authenticode_status": str(observed["authenticode_status"]),
            "release_metadata_path": str(release_metadata),
            "release_metadata_sha256": _artifact(release_metadata)["sha256"],
            "release_version": str(binary_release["version"]),
        },
        "fixture": {
            "nonce": nonce,
            "script": _artifact(fixture_script),
            "command_sha256": _sha256_bytes(command.encode("utf-8")),
            "tool_timeout_ms": args.tool_timeout_ms,
            "hold_seconds": 300,
        },
        "process_tree": {
            "roles": role_records,
            "role_artifacts": [_artifact(path) for path in role_paths],
            "pre_trigger_processes": pre_trigger_processes,
            "post_trigger_processes": post_trigger_processes,
            "parent_exit_triggered": parent_exit_triggered,
            "residue_zero": len(post_trigger_processes) == 0,
        },
        "grok": {
            "pid": grok.pid if grok is not None else None,
            "exit_code": grok.returncode if grok is not None else None,
            "outer_job_created": job is not None,
            "outer_job_assigned": job is not None and job.assigned,
            "outer_job_closed": job is not None and job.closed,
        },
        "provider": {
            "scenario": provider_scenario,
            "primary_request_count": len(primary),
            "expected_primary_request_count": expected_request_count,
            "tool_sequence": tool_sequence,
            "expected_tool_sequence": expected_tool_sequence,
            "real_model_invoked": False,
            "private_capture": _artifact(private_capture_path),
            "result": _artifact(provider_result_path),
        },
        "capture": {
            "grok_stdout": _artifact(grok_stdout_path),
            "grok_stderr": _artifact(grok_stderr_path),
            "provider_stdout": _artifact(provider_stdout_path),
            "provider_stderr": _artifact(provider_stderr_path),
            "marker_projection": marker_projection,
            "output_drain_required": output_drain_required,
            "output_drain_observed": output_drain_observed,
        },
        "workspace": {
            "preflight": _artifact(preflight_path),
            "launch": _artifact(launch_path),
            "postrun": _artifact(postrun_path),
            "control_aggregate_sha256": preflight["discovery"]["aggregate_sha256"],
            "scan_policy_sha256": preflight["discovery"]["scan_policy_sha256"],
        },
        "firewall": {
            "receipt": _artifact(firewall_receipt_path),
            "removed": firewall_receipt.get("removed") is True,
            "remaining_rule_count": firewall_receipt.get("remaining_rule_count"),
        },
        "checks": checks,
        "limitations": [
            "This is a fake-only Windows containment conformance probe, not a real model invocation.",
            "A zero post-trigger nonce scan establishes no matching live command line at the observation time; it is not a timeless OS guarantee.",
            "The parent-exit scenario requires process-tree records but does not require tool-output drain because the owning process is deliberately terminated.",
            "Mechanical PASS does not establish scientific correctness or promote any FEP/LIF claim.",
        ],
    }
    _atomic_write_json(result_path, result)
    completed = subprocess.run(
        [
            str(python_path),
            str(verify_script),
            "--result",
            str(result_path),
            "--output",
            str(verification_path),
        ],
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        encoding="utf-8",
        errors="replace",
        timeout=60,
        check=False,
        creationflags=CREATE_NO_WINDOW,
    )
    if completed.returncode != 0:
        raise RuntimeError(f"independent verifier failed: {completed.stderr.strip()}")
    return result


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Run a fake-only Grok Windows child-tree containment probe"
    )
    parser.add_argument("--output-directory", type=Path, required=True)
    parser.add_argument(
        "--scenario",
        choices=tuple(SCENARIO_TO_PROVIDER),
        required=True,
    )
    parser.add_argument("--release-metadata", type=Path, required=True)
    parser.add_argument("--binary", type=Path)
    parser.add_argument("--python-path", type=Path)
    parser.add_argument("--tool-timeout-ms", type=int, default=3000)
    parser.add_argument("--timeout-seconds", type=int, default=60)
    parser.add_argument("--exit-grace-seconds", type=int, default=10)
    args = parser.parse_args()
    if args.tool_timeout_ms < 1000 or args.tool_timeout_ms > 30000:
        parser.error("--tool-timeout-ms must be in [1000, 30000]")
    if args.timeout_seconds < 20 or args.timeout_seconds > 180:
        parser.error("--timeout-seconds must be in [20, 180]")
    if args.exit_grace_seconds < 0 or args.exit_grace_seconds > 30:
        parser.error("--exit-grace-seconds must be in [0, 30]")
    try:
        result = run(args)
    except Exception as error:
        print(f"{type(error).__name__}: {error}", file=sys.stderr)
        return 1
    print(json.dumps(result, ensure_ascii=False, sort_keys=True, allow_nan=False))
    return 0 if result["valid"] else 2


if __name__ == "__main__":
    raise SystemExit(main())
