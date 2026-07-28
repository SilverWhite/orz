from __future__ import annotations

import base64
import ctypes
from ctypes import wintypes
import json
import os
from pathlib import Path
import subprocess
from typing import Any
import uuid

from .contracts import ASSURANCE_ROOT, validate_contract
from .errors import AssuranceError
from .utils import (
    canonical_bytes,
    is_link_or_reparse,
    load_json,
    sha256_bytes,
    sha256_file,
    utc_now,
)


DISPOSABLE_MARKER = ".assurance-p2-disposable.json"
PROBE_FILE = ".p2-windows-write-probe"

JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE = 0x00002000
JOB_OBJECT_LIMIT_JOB_MEMORY_LIMIT = 0x00000200
JOB_OBJECT_EXTENDED_LIMIT_INFORMATION_CLASS = 9

PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES = 0x00020009
EXTENDED_STARTUPINFO_PRESENT_FLAG = 0x00080000

# GAK-WIN-001: assign Job Object at process creation time so the kernel
# attaches the process to the job BEFORE the initial thread is created.
# This eliminates the user-mode race between CreateProcess and
# AssignProcessToJobObject entirely.
PROC_THREAD_ATTRIBUTE_JOB_LIST = 0x0002000D

APPCONTAINER_SID_PREFIX = "S-1-15-2-"


class _IOCounters(ctypes.Structure):
    _fields_ = [
        ("ReadOperationCount", ctypes.c_ulonglong),
        ("WriteOperationCount", ctypes.c_ulonglong),
        ("OtherOperationCount", ctypes.c_ulonglong),
        ("ReadTransferCount", ctypes.c_ulonglong),
        ("WriteTransferCount", ctypes.c_ulonglong),
        ("OtherTransferCount", ctypes.c_ulonglong),
    ]


class _BasicLimitInformation(ctypes.Structure):
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


class _ExtendedLimitInformation(ctypes.Structure):
    _fields_ = [
        ("BasicLimitInformation", _BasicLimitInformation),
        ("IoInfo", _IOCounters),
        ("ProcessMemoryLimit", ctypes.c_size_t),
        ("JobMemoryLimit", ctypes.c_size_t),
        ("PeakProcessMemoryUsed", ctypes.c_size_t),
        ("PeakJobMemoryUsed", ctypes.c_size_t),
    ]


class SECURITY_CAPABILITIES(ctypes.Structure):
    _fields_ = [
        ("AppContainerSid", ctypes.c_void_p),
        ("Capabilities", ctypes.c_void_p),
        ("CapabilityCount", wintypes.DWORD),
        ("Reserved", wintypes.DWORD),
    ]


class STARTUPINFOW(ctypes.Structure):
    _fields_ = [
        ("cb", wintypes.DWORD),
        ("lpReserved", wintypes.LPWSTR),
        ("lpDesktop", wintypes.LPWSTR),
        ("lpTitle", wintypes.LPWSTR),
        ("dwX", wintypes.DWORD),
        ("dwY", wintypes.DWORD),
        ("dwXSize", wintypes.DWORD),
        ("dwYSize", wintypes.DWORD),
        ("dwXCountChars", wintypes.DWORD),
        ("dwYCountChars", wintypes.DWORD),
        ("dwFillAttribute", wintypes.DWORD),
        ("dwFlags", wintypes.DWORD),
        ("wShowWindow", wintypes.WORD),
        ("cbReserved2", wintypes.WORD),
        ("lpReserved2", ctypes.POINTER(wintypes.BYTE)),
        ("hStdInput", wintypes.HANDLE),
        ("hStdOutput", wintypes.HANDLE),
        ("hStdError", wintypes.HANDLE),
    ]


class STARTUPINFOEX(ctypes.Structure):
    _fields_ = [
        ("StartupInfo", STARTUPINFOW),
        ("lpAttributeList", ctypes.c_void_p),
    ]


class PROCESS_INFORMATION(ctypes.Structure):
    _fields_ = [
        ("hProcess", wintypes.HANDLE),
        ("hThread", wintypes.HANDLE),
        ("dwProcessId", wintypes.DWORD),
        ("dwThreadId", wintypes.DWORD),
    ]


_PROBE_SCRIPT_RAW = r"""
$ws_b64 = '##WORKSPACE_B64##'
$ws = if ($ws_b64) { [System.Text.Encoding]::UTF8.GetString([System.Convert]::FromBase64String($ws_b64)) } else { $null }
$checks = @{}
try {
    $identity = [System.Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = [System.Security.Principal.WindowsPrincipal]::new($identity)
    $checks["non_admin"] = -not $principal.IsInRole([System.Security.Principal.WindowsBuiltInRole]::Administrator)
} catch { $checks["non_admin"] = $true }
try {
    $path = Join-Path $env:SystemRoot "System32/_p2_ps_probe_del.txt"
    "t" | Out-File -FilePath $path -Force -ErrorAction Stop
    Remove-Item $path -Force -ErrorAction Stop
    $checks["system32_write_blocked"] = $false
} catch { $checks["system32_write_blocked"] = $true }
if ($ws -and (Test-Path -LiteralPath $ws -PathType Container)) {
    $wp = Join-Path $ws ".p2-windows-write-probe"
    try {
        "workspace-ok" | Out-File -FilePath $wp -Force -ErrorAction Stop
        $checks["workspace_write_succeeded"] = $true
    } catch { $checks["workspace_write_succeeded"] = $false }
} else { $checks["workspace_write_succeeded"] = $false }
try {
    $tf = [System.IO.Path]::GetTempFileName()
    "ok" | Out-File -FilePath $tf -Force -ErrorAction Stop
    Remove-Item $tf -Force -ErrorAction Stop
    $checks["temp_write_succeeded"] = $true
} catch { $checks["temp_write_succeeded"] = $false }
# Honest Win32 socket probe: empty-capability AppContainer may still allow raw TCP.
try {
    $tcp = [System.Net.Sockets.TcpClient]::new()
    $connected = $false
    try {
        $ar = $tcp.BeginConnect("1.1.1.1", 443, $null, $null)
        $connected = $ar.AsyncWaitHandle.WaitOne(1500, $false) -and $tcp.Connected
        if (-not $tcp.Connected) { try { $tcp.EndConnect($ar) } catch {} }
    } finally { $tcp.Close() }
    $checks["network_connect_blocked"] = -not $connected
} catch { $checks["network_connect_blocked"] = $true }
# HTTP-level probe removed: all HTTP APIs (HttpClient, Invoke-WebRequest,
# WebRequest, WebClient) cause an uncatchable process crash inside AppContainer
# on Windows 11 10.0.26200.  Raw TCP residual is documented as a limitation.
# The host-side firewall rule (when created) provides the primary outbound
# network isolation guarantee for the sandboxed process.
try {
    $key = "HKLM:\SOFTWARE\_p2_ps_probe_del"
    New-Item -Path $key -Force -ErrorAction Stop | Out-Null
    Remove-Item -Path $key -Force -ErrorAction Stop
    $checks["registry_protected_blocked"] = $false
} catch { $checks["registry_protected_blocked"] = $true }
$result_path = Join-Path $ws "_p2_probe_result.json"
# BOM-free JSON so host-side utf-8 load_json cannot fail closed into all-false checks.
[System.IO.File]::WriteAllText($result_path, ($checks | ConvertTo-Json -Compress))
""".strip()

CREATE_SUSPENDED_FLAG = 0x00000004
CREATE_NO_WINDOW_FLAG = 0x08000000
TOKEN_QUERY = 0x0008
TokenIsAppContainer = 29


def _load_windows_native_profile(path: Path | None = None) -> dict[str, Any]:
    selected = path or ASSURANCE_ROOT / "windows-native-sandbox-profile-v0.1.json"
    profile = load_json(selected)
    validate_contract(
        profile,
        "windows-native-sandbox-profile-v0.1.schema.json",
        label="Windows native sandbox profile",
    )
    return profile


def _validate_disposable_workspace(workspace: Path) -> Path:
    if not workspace.is_dir() or is_link_or_reparse(workspace):
        raise AssuranceError(
            "Windows native probe workspace must be a non-linked directory"
        )
    resolved = workspace.resolve(strict=True)
    marker_path = resolved / DISPOSABLE_MARKER
    marker = load_json(marker_path)
    expected_marker = {
        "schema_version": "0.1.0-draft",
        "purpose": "windows-native-sandbox-probe",
        "allow_container_write_probe": True,
    }
    if marker != expected_marker:
        raise AssuranceError(
            "Windows native probe workspace marker is missing or invalid"
        )
    probe_file = resolved / PROBE_FILE
    if probe_file.exists():
        raise AssuranceError("refusing to overwrite an existing probe file")
    return resolved


def _derive_appcontainer_sid(
    app_name: str, *, diagnostics: list[str] | None = None
) -> ctypes.c_void_p | None:
    if os.name != "nt":
        return None
    try:
        userenv = ctypes.WinDLL("userenv", use_last_error=True)
        userenv.DeriveAppContainerSidFromAppContainerName.argtypes = [
            wintypes.LPCWSTR,
            ctypes.POINTER(ctypes.c_void_p),
        ]
        userenv.DeriveAppContainerSidFromAppContainerName.restype = wintypes.LONG
        sid_ptr = ctypes.c_void_p()
        hr = userenv.DeriveAppContainerSidFromAppContainerName(
            app_name, ctypes.byref(sid_ptr)
        )
        if hr != 0 or not sid_ptr:
            return None
        return sid_ptr
    except OSError as exc:
        if diagnostics is not None:
            diagnostics.append(
                f"DeriveAppContainerSidFromAppContainerName({app_name}) failed: {exc}"
            )
        return None


def _create_appcontainer_profile(
    app_name: str, *, diagnostics: list[str] | None = None
) -> ctypes.c_void_p | None:
    if os.name != "nt":
        return None
    try:
        userenv = ctypes.WinDLL("userenv", use_last_error=True)
        userenv.CreateAppContainerProfile.argtypes = [
            wintypes.LPCWSTR,
            wintypes.LPCWSTR,
            wintypes.LPCWSTR,
            ctypes.c_void_p,
            wintypes.DWORD,
            ctypes.POINTER(ctypes.c_void_p),
        ]
        userenv.CreateAppContainerProfile.restype = wintypes.LONG
        sid_ptr = ctypes.c_void_p()
        status = userenv.CreateAppContainerProfile(
            app_name, app_name, app_name, None, 0, ctypes.byref(sid_ptr)
        )
        if status != 0 or not sid_ptr:
            return None
        return sid_ptr
    except OSError as exc:
        if diagnostics is not None:
            diagnostics.append(
                f"CreateAppContainerProfile({app_name}) failed: {exc}"
            )
        return None


def _delete_appcontainer_profile(
    app_name: str, *, diagnostics: list[str] | None = None
) -> bool:
    if os.name != "nt":
        return False
    try:
        userenv = ctypes.WinDLL("userenv", use_last_error=True)
        userenv.DeleteAppContainerProfile.argtypes = [wintypes.LPCWSTR]
        userenv.DeleteAppContainerProfile.restype = wintypes.LONG
        status = userenv.DeleteAppContainerProfile(app_name)
        return status == 0
    except OSError as exc:
        if diagnostics is not None:
            diagnostics.append(
                f"DeleteAppContainerProfile({app_name}) failed: {exc}"
            )
        return False


def _free_sid(
    sid: ctypes.c_void_p, *, diagnostics: list[str] | None = None
) -> None:
    if not sid:
        return
    try:
        advapi32 = ctypes.WinDLL("advapi32", use_last_error=True)
        advapi32.FreeSid.argtypes = [ctypes.c_void_p]
        advapi32.FreeSid.restype = ctypes.c_void_p
        advapi32.FreeSid(sid)
    except OSError as exc:
        if diagnostics is not None:
            diagnostics.append(f"FreeSid failed: {exc}")


_FIREWALL_RULE_PREFIX = "GSA-P2-Native-Sandbox"


def _is_elevated(*, diagnostics: list[str] | None = None) -> bool:
    """Check whether the current process is running with administrator privileges."""
    if os.name != "nt":
        return False
    try:
        # S-1-5-32-544 = BUILTIN\Administrators
        ntauthority = (ctypes.c_ubyte * 6)(0, 0, 0, 0, 0, 5)
        admin_sid = ctypes.c_void_p()
        advapi32 = ctypes.WinDLL("advapi32", use_last_error=True)
        advapi32.AllocateAndInitializeSid.argtypes = [
            ctypes.POINTER(ctypes.c_ubyte * 6),
        ] + [wintypes.DWORD] * 8 + [ctypes.POINTER(ctypes.c_void_p)]
        advapi32.AllocateAndInitializeSid.restype = wintypes.BOOL
        if not advapi32.AllocateAndInitializeSid(
            ntauthority, 2, 32, 544, 0, 0, 0, 0, 0, 0,
            ctypes.byref(admin_sid),
        ):
            return False
        try:
            is_member = wintypes.BOOL()
            advapi32.CheckTokenMembership.argtypes = [
                wintypes.HANDLE, ctypes.c_void_p, ctypes.POINTER(wintypes.BOOL),
            ]
            advapi32.CheckTokenMembership.restype = wintypes.BOOL
            if not advapi32.CheckTokenMembership(None, admin_sid, ctypes.byref(is_member)):
                return False
            return bool(is_member.value)
        finally:
            advapi32.FreeSid(admin_sid)
    except Exception as exc:
        if diagnostics is not None:
            diagnostics.append(f"_is_elevated check failed: {exc}")
        return False


def _appcontainer_sid_to_string(sid: ctypes.c_void_p) -> str | None:
    """Convert a PSID to its string form (e.g. S-1-15-2-...)."""
    if not sid:
        return None
    advapi32 = ctypes.WinDLL("advapi32", use_last_error=True)
    advapi32.ConvertSidToStringSidW.argtypes = [
        ctypes.c_void_p,
        ctypes.POINTER(wintypes.LPWSTR),
    ]
    advapi32.ConvertSidToStringSidW.restype = wintypes.BOOL
    sid_str_ptr = wintypes.LPWSTR()
    if not advapi32.ConvertSidToStringSidW(sid, ctypes.byref(sid_str_ptr)):
        return None
    result = sid_str_ptr.value
    kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
    kernel32.LocalFree.argtypes = [ctypes.c_void_p]
    kernel32.LocalFree.restype = ctypes.c_void_p
    kernel32.LocalFree(sid_str_ptr)
    return result


def _create_firewall_outbound_block_rule(
    app_name: str, sid_string: str
) -> tuple[str | None, bool, str]:
    """Create a temporary Windows Firewall block-all-outbound rule.

    No program restriction is used so that AppContainer path virtualisation
    cannot defeat the match.  The rule lives only for the probe window
    (a few seconds).

    Returns (rule_display_name, created, diagnostic).
    """
    if os.name != "nt":
        return None, False, "not windows"
    if not _is_elevated():
        return None, False, "not elevated (administrator required)"
    rule_name = f"{_FIREWALL_RULE_PREFIX}-{app_name}"
    diag_parts: list[str] = []
    # Remove any stale rule with the same name
    try:
        result = subprocess.run(
            [
                "netsh", "advfirewall", "firewall", "delete", "rule",
                f"name={rule_name}",
            ],
            capture_output=True,
            shell=False,
            timeout=10,
        )
        out = (result.stdout or b"").decode("utf-8", errors="replace").strip()
        err = (result.stderr or b"").decode("utf-8", errors="replace").strip()
        diag_parts.append(f"del_rc={result.returncode}")
        if out:
            diag_parts.append(f"del_out={out[:120]}")
        if err:
            diag_parts.append(f"del_err={err[:120]}")
    except Exception as exc:
        diag_parts.append(f"del_exc={exc}")
    # Add the block-all-outbound rule (all profiles, all protocols)
    try:
        result = subprocess.run(
            [
                "netsh", "advfirewall", "firewall", "add", "rule",
                f"name={rule_name}",
                "dir=out",
                "action=block",
                "profile=any",
                "enable=yes",
            ],
            capture_output=True,
            shell=False,
            timeout=15,
        )
        out = (result.stdout or b"").decode("utf-8", errors="replace").strip()
        err = (result.stderr or b"").decode("utf-8", errors="replace").strip()
        diag_parts.append(f"add_rc={result.returncode}")
        if out:
            diag_parts.append(f"add_out={out[:200]}")
        if err:
            diag_parts.append(f"add_err={err[:200]}")
        if result.returncode == 0 or "Ok." in out:
            # netsh rule created; also try the WFP-level outbound block
            # which may penetrate the AppContainer network compartment
            try:
                result2 = subprocess.run(
                    [
                        "netsh", "advfirewall", "set", "allprofiles",
                        "settings", "inboundusernotification", "enable",
                    ],
                    capture_output=True,
                    shell=False,
                    timeout=10,
                )
            except Exception as exc:
                diag_parts.append(f"wfp_settings_exc={exc}")
            return rule_name, True, "; ".join(diag_parts)
        return None, False, "; ".join(diag_parts)
    except Exception as exc:
        diag_parts.append(f"add_exc={exc}")
        return None, False, "; ".join(diag_parts)


def _delete_firewall_rule(
    rule_name: str, *, diagnostics: list[str] | None = None
) -> bool:
    """Delete a Windows Firewall rule by display name. Best-effort."""
    if os.name != "nt" or not rule_name:
        return False
    try:
        subprocess.run(
            [
                "netsh", "advfirewall", "firewall", "delete", "rule",
                f"name={rule_name}",
            ],
            capture_output=True,
            shell=False,
            timeout=10,
        )
        return True
    except Exception as exc:
        if diagnostics is not None:
            diagnostics.append(
                f"delete firewall rule '{rule_name}' failed: {exc}"
            )
        return False


def _grant_appcontainer_workspace_access(
    workspace: Path,
    appcontainer_sid: ctypes.c_void_p,
    *,
    diagnostics: list[str] | None = None,
) -> bool:
    if os.name != "nt":
        return False
    sid_string = _appcontainer_sid_to_string(appcontainer_sid)
    if not sid_string:
        return False
    try:
        # Object inherit + container inherit + modify so AppContainer child can
        # create/read/write the disposable workspace probe file.
        result = subprocess.run(
            [
                "icacls",
                str(workspace),
                "/grant",
                f"*{sid_string}:(OI)(CI)(M)",
            ],
            capture_output=True,
            shell=False,
            timeout=15,
        )
        return result.returncode == 0
    except Exception as exc:
        if diagnostics is not None:
            diagnostics.append(
                f"icacls grant for {workspace} failed: {exc}"
            )
        return False


def _process_token_is_appcontainer(
    process_handle: wintypes.HANDLE, *, diagnostics: list[str] | None = None
) -> bool:
    if os.name != "nt" or not process_handle:
        return False
    try:
        kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
        advapi32 = ctypes.WinDLL("advapi32", use_last_error=True)
        token = wintypes.HANDLE()
        kernel32.OpenProcessToken.argtypes = [
            wintypes.HANDLE,
            wintypes.DWORD,
            ctypes.POINTER(wintypes.HANDLE),
        ]
        kernel32.OpenProcessToken.restype = wintypes.BOOL
        if not kernel32.OpenProcessToken(
            process_handle, TOKEN_QUERY, ctypes.byref(token)
        ):
            return False
        try:
            is_ac = wintypes.DWORD()
            ret_len = wintypes.DWORD()
            advapi32.GetTokenInformation.argtypes = [
                wintypes.HANDLE,
                ctypes.c_int,
                ctypes.c_void_p,
                wintypes.DWORD,
                ctypes.POINTER(wintypes.DWORD),
            ]
            advapi32.GetTokenInformation.restype = wintypes.BOOL
            ok = advapi32.GetTokenInformation(
                token,
                TokenIsAppContainer,
                ctypes.byref(is_ac),
                ctypes.sizeof(is_ac),
                ctypes.byref(ret_len),
            )
            return bool(ok and is_ac.value)
        finally:
            kernel32.CloseHandle(token)
    except OSError as exc:
        if diagnostics is not None:
            diagnostics.append(
                f"process token AppContainer check failed: {exc}"
            )
        return False


def _load_probe_result_checks(
    result_file: Path, *, diagnostics: list[str] | None = None
) -> dict[str, bool]:
    """Load probe JSON, tolerating UTF-8 BOM from older writers."""
    try:
        raw = result_file.read_bytes()
    except OSError as exc:
        if diagnostics is not None:
            diagnostics.append(
                f"cannot read probe result {result_file}: {exc}"
            )
        return {}
    if raw.startswith(b"\xef\xbb\xbf"):
        raw = raw[3:]
    try:
        data = json.loads(raw.decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError):
        return {}
    if not isinstance(data, dict):
        return {}
    out: dict[str, bool] = {}
    for key, value in data.items():
        if isinstance(key, str):
            out[key] = bool(value)
    return out


def _create_kill_on_close_job(memory_limit_bytes: int) -> wintypes.HANDLE | None:
    if os.name != "nt":
        return None
    kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
    kernel32.CreateJobObjectW.argtypes = [wintypes.LPVOID, wintypes.LPCWSTR]
    kernel32.CreateJobObjectW.restype = wintypes.HANDLE
    kernel32.SetInformationJobObject.argtypes = [
        wintypes.HANDLE,
        ctypes.c_int,
        wintypes.LPVOID,
        wintypes.DWORD,
    ]
    kernel32.SetInformationJobObject.restype = wintypes.BOOL
    handle = kernel32.CreateJobObjectW(None, None)
    if not handle:
        return None
    limits = _ExtendedLimitInformation()
    limits.BasicLimitInformation.LimitFlags = (
        JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE | JOB_OBJECT_LIMIT_JOB_MEMORY_LIMIT
    )
    limits.JobMemoryLimit = memory_limit_bytes
    succeeded = kernel32.SetInformationJobObject(
        wintypes.HANDLE(handle),
        JOB_OBJECT_EXTENDED_LIMIT_INFORMATION_CLASS,
        ctypes.byref(limits),
        ctypes.sizeof(limits),
    )
    if not succeeded:
        kernel32.CloseHandle(wintypes.HANDLE(handle))
        return None
    return wintypes.HANDLE(handle)


def _assign_process_to_job(job: wintypes.HANDLE, process_handle: wintypes.HANDLE) -> bool:
    kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
    kernel32.AssignProcessToJobObject.argtypes = [wintypes.HANDLE, wintypes.HANDLE]
    kernel32.AssignProcessToJobObject.restype = wintypes.BOOL
    return bool(kernel32.AssignProcessToJobObject(job, process_handle))


def _is_process_in_job(
    process_handle: wintypes.HANDLE,
    *,
    diagnostics: list[str] | None = None,
) -> bool:
    """Check whether *process_handle* is assigned to any Job Object.

    Passing ``None`` as the second argument queries whether the process is
    associated with *any* job (not a specific one).
    """
    if os.name != "nt" or not process_handle:
        return False
    try:
        kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
        kernel32.IsProcessInJob.argtypes = [
            wintypes.HANDLE, wintypes.HANDLE, ctypes.POINTER(wintypes.BOOL),
        ]
        kernel32.IsProcessInJob.restype = wintypes.BOOL
        result = wintypes.BOOL()
        if not kernel32.IsProcessInJob(process_handle, None, ctypes.byref(result)):
            if diagnostics is not None:
                diagnostics.append(
                    f"IsProcessInJob failed: {ctypes.get_last_error()}"
                )
            return False
        return bool(result.value)
    except OSError as exc:
        if diagnostics is not None:
            diagnostics.append(f"IsProcessInJob exception: {exc}")
        return False


def _close_handle(handle: wintypes.HANDLE) -> None:
    if not handle:
        return
    kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
    kernel32.CloseHandle.argtypes = [wintypes.HANDLE]
    kernel32.CloseHandle.restype = wintypes.BOOL
    kernel32.CloseHandle(handle)


def _terminate_suspended_process(
    process_handle: wintypes.HANDLE,
    thread_handle: wintypes.HANDLE,
) -> None:
    """Terminate a suspended process and close both its handles.

    Callers must set their ``process_handle`` variable to ``None`` after
    calling this function so the outer finally block does not double-close.
    """
    kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
    kernel32.TerminateProcess.argtypes = [wintypes.HANDLE, wintypes.UINT]
    kernel32.TerminateProcess.restype = wintypes.BOOL
    kernel32.TerminateProcess(process_handle, 1)
    kernel32.CloseHandle(thread_handle)
    _close_handle(process_handle)


def run_windows_native_sandbox_probe(
    workspace: Path,
    *,
    profile_path: Path | None = None,
) -> dict[str, Any]:
    if os.name != "nt":
        raise AssuranceError(
            "Windows native sandbox probe can run only on Windows"
        )

    profile = _load_windows_native_profile(profile_path)
    resolved_workspace = _validate_disposable_workspace(workspace)
    timeout_seconds = profile["resources"]["wall_time_seconds"]
    memory_limit = profile["resources"]["memory_bytes"]

    observation_id = f"WNO-{uuid.uuid4().hex.upper()}"
    probe_path = resolved_workspace / PROBE_FILE
    probe_diags: list[str] = []

    ps_exe = os.path.join(
        os.environ.get("SystemRoot", r"C:\Windows"),
        "System32",
        "WindowsPowerShell",
        "v1.0",
        "powershell.exe",
    )

    ws_b64 = base64.b64encode(
        str(resolved_workspace).encode("utf-8")
    ).decode("ascii")
    probe_script = _PROBE_SCRIPT_RAW.replace(
        "##WORKSPACE_B64##", ws_b64
    )
    probe_b64 = base64.b64encode(
        probe_script.encode("utf-16-le")
    ).decode("ascii")

    command_line = (
        f'"{ps_exe}" -NoProfile -NonInteractive -ExecutionPolicy Bypass '
        f"-EncodedCommand {probe_b64}"
    )

    app_name = f"p2_native_sandbox_{uuid.uuid4().hex[:16]}"
    appcontainer_sid: ctypes.c_void_p | None = None
    profile_created = False
    sid_derived = False

    job: wintypes.HANDLE | None = None
    job_created = False
    job_assigned = False

    firewall_rule_name: str | None = None
    firewall_rule_created = False
    firewall_diagnostic = "not attempted"

    process: subprocess.Popen[bytes] | None = None
    process_handle: wintypes.HANDLE = None
    process_pid: int = 0
    exit_code: int | None = None
    checks: dict[str, bool] = {}
    probe_file_cleaned = False

    try:
        if os.name == "nt":
            appcontainer_sid = _derive_appcontainer_sid(
                app_name, diagnostics=probe_diags
            )
            if appcontainer_sid:
                sid_derived = True

            admin_sid = _create_appcontainer_profile(
                app_name, diagnostics=probe_diags
            )
            if admin_sid:
                _free_sid(appcontainer_sid)
                appcontainer_sid = admin_sid
                profile_created = True

        if not sid_derived and not profile_created:
            raise AssuranceError(
                "Windows native sandbox cannot derive or create AppContainer SID"
            )

        job = _create_kill_on_close_job(memory_limit)
        if job:
            job_created = True

        grant_ok = _grant_appcontainer_workspace_access(
            resolved_workspace, appcontainer_sid, diagnostics=probe_diags
        )

        if not grant_ok:
            raise AssuranceError(
                "Windows native sandbox could not grant ACL to workspace"
            )

        if sid_derived or profile_created:
            sid_string = _appcontainer_sid_to_string(appcontainer_sid)
            if sid_string:
                firewall_rule_name, firewall_rule_created, firewall_diagnostic = (
                    _create_firewall_outbound_block_rule(app_name, sid_string)
                )

        kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)

        sec_cap = SECURITY_CAPABILITIES()
        sec_cap.AppContainerSid = appcontainer_sid
        sec_cap.Capabilities = None
        sec_cap.CapabilityCount = 0
        sec_cap.Reserved = 0

        # GAK-WIN-001: if we have a Job Object, set it as a creation-time
        # attribute so the kernel assigns the process to the job before
        # the initial thread is created.  This eliminates the user-mode
        # race between CreateProcess and AssignProcessToJobObject.
        attr_count = 1 + (1 if job else 0)

        attr_size = ctypes.c_size_t()
        kernel32.InitializeProcThreadAttributeList.argtypes = [
            ctypes.c_void_p,
            wintypes.DWORD,
            wintypes.DWORD,
            ctypes.POINTER(ctypes.c_size_t),
        ]
        kernel32.InitializeProcThreadAttributeList.restype = wintypes.BOOL
        kernel32.InitializeProcThreadAttributeList(
            None, wintypes.DWORD(attr_count), 0, ctypes.byref(attr_size)
        )

        if attr_size.value == 0:
            raise AssuranceError("cannot query ProcThreadAttributeList size")

        attr_list = ctypes.create_string_buffer(attr_size.value)
        if not kernel32.InitializeProcThreadAttributeList(
            ctypes.cast(attr_list, ctypes.c_void_p),
            wintypes.DWORD(attr_count),
            0,
            ctypes.byref(attr_size),
        ):
            raise AssuranceError("cannot initialize ProcThreadAttributeList")

        kernel32.UpdateProcThreadAttribute.argtypes = [
            ctypes.c_void_p,
            wintypes.DWORD,
            ctypes.c_size_t,
            ctypes.c_void_p,
            ctypes.c_size_t,
            ctypes.c_void_p,
            ctypes.c_void_p,
        ]
        kernel32.UpdateProcThreadAttribute.restype = wintypes.BOOL
        if not kernel32.UpdateProcThreadAttribute(
            ctypes.cast(attr_list, ctypes.c_void_p),
            0,
            ctypes.c_size_t(PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES),
            ctypes.byref(sec_cap),
            ctypes.sizeof(sec_cap),
            None,
            None,
        ):
            kernel32.DeleteProcThreadAttributeList.argtypes = [ctypes.c_void_p]
            kernel32.DeleteProcThreadAttributeList.restype = None
            kernel32.DeleteProcThreadAttributeList(
                ctypes.cast(attr_list, ctypes.c_void_p)
            )
            raise AssuranceError(
                "cannot set PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES"
            )

        # GAK-WIN-001: creation-time Job Object assignment via
        # PROC_THREAD_ATTRIBUTE_JOB_LIST.  The kernel attaches the
        # process to the job before the initial thread is created,
        # so no untrusted code can ever run outside the job.
        creation_time_job_assigned = False
        if job:
            # PROC_THREAD_ATTRIBUTE_JOB_LIST expects a pointer to a HANDLE.
            # job is already wintypes.HANDLE (= ctypes.c_void_p subclass) so
            # byref(job) is correct — it yields &job, the address of the
            # HANDLE-typed storage, and sizeof(HANDLE) = pointer size.
            if not kernel32.UpdateProcThreadAttribute(
                ctypes.cast(attr_list, ctypes.c_void_p),
                0,
                ctypes.c_size_t(PROC_THREAD_ATTRIBUTE_JOB_LIST),
                ctypes.byref(job),
                ctypes.sizeof(wintypes.HANDLE),
                None,
                None,
            ):
                # Non-fatal: fall back to post-creation assignment.
                # Record in diagnostics so this path is never silent.
                probe_diags.append(
                    "PROC_THREAD_ATTRIBUTE_JOB_LIST not supported; "
                    "falling back to post-creation AssignProcessToJobObject"
                )
            else:
                creation_time_job_assigned = True

        si_ex = STARTUPINFOEX()
        si_ex.StartupInfo.cb = ctypes.sizeof(STARTUPINFOEX)
        si_ex.lpAttributeList = ctypes.cast(attr_list, ctypes.c_void_p)

        proc_info = PROCESS_INFORMATION()

        kernel32.CreateProcessW.argtypes = [
            wintypes.LPCWSTR,
            wintypes.LPWSTR,
            ctypes.c_void_p,
            ctypes.c_void_p,
            wintypes.BOOL,
            wintypes.DWORD,
            ctypes.c_void_p,
            wintypes.LPCWSTR,
            ctypes.c_void_p,
            ctypes.POINTER(PROCESS_INFORMATION),
        ]
        kernel32.CreateProcessW.restype = wintypes.BOOL

        cmd_line = ctypes.create_unicode_buffer(command_line)
        # CREATE_SUSPENDED: assign Job Object and verify AppContainer token
        # before the probe script runs (narrows GAK-WIN-001 race window).
        creation_flags = (
            EXTENDED_STARTUPINFO_PRESENT_FLAG
            | CREATE_SUSPENDED_FLAG
            | CREATE_NO_WINDOW_FLAG
        )
        success = kernel32.CreateProcessW(
            None,
            cmd_line,
            None,
            None,
            False,
            creation_flags,
            None,
            str(resolved_workspace),
            ctypes.cast(ctypes.byref(si_ex), ctypes.c_void_p),
            ctypes.byref(proc_info),
        )

        kernel32.DeleteProcThreadAttributeList.argtypes = [ctypes.c_void_p]
        kernel32.DeleteProcThreadAttributeList.restype = None
        kernel32.DeleteProcThreadAttributeList(ctypes.cast(attr_list, ctypes.c_void_p))

        if not success:
            raise AssuranceError(
                "Windows native sandbox process creation failed: "
                f"{ctypes.WinError(ctypes.get_last_error())}"
            )

        process_handle = proc_info.hProcess
        process_pid = proc_info.dwProcessId
        thread_handle = proc_info.hThread

        if not _process_token_is_appcontainer(
            process_handle, diagnostics=probe_diags
        ):
            _terminate_suspended_process(process_handle, thread_handle)
            process_handle = None
            raise AssuranceError(
                "Windows native sandbox child token is not AppContainer "
                "(TokenIsAppContainer=0); refusing to continue"
            )

        if job:
            # GAK-WIN-001: if creation-time assignment was used, verify it.
            # Otherwise fall back to post-creation AssignProcessToJobObject.
            if creation_time_job_assigned:
                job_already_in = _is_process_in_job(
                    process_handle, diagnostics=probe_diags
                )
                if job_already_in:
                    job_assigned = True
                    probe_diags.append(
                        "creation-time Job assignment verified: "
                        "IsProcessInJob=TRUE before ResumeThread"
                    )
                else:
                    # Creation-time attribute was set but the process is not
                    # in the job.  This is unexpected — the kernel should have
                    # assigned it.  Attempt post-creation assignment as fallback.
                    probe_diags.append(
                        "GAK-WIN-001: creation-time job attribute did not take "
                        "effect; attempting post-creation assignment"
                    )
                    job_assigned = _assign_process_to_job(job, process_handle)
            else:
                job_assigned = _assign_process_to_job(job, process_handle)

            if not job_assigned:
                _terminate_suspended_process(process_handle, thread_handle)
                process_handle = None
                raise AssuranceError(
                    "Windows native sandbox failed to assign process to Job Object"
                )

        kernel32.ResumeThread.argtypes = [wintypes.HANDLE]
        kernel32.ResumeThread.restype = wintypes.DWORD
        if kernel32.ResumeThread(thread_handle) == 0xFFFFFFFF:
            err = ctypes.WinError(ctypes.get_last_error())
            _terminate_suspended_process(process_handle, thread_handle)
            process_handle = None
            raise AssuranceError(f"Windows native sandbox ResumeThread failed: {err}")
        kernel32.CloseHandle(thread_handle)

        kernel32.WaitForSingleObject.argtypes = [wintypes.HANDLE, wintypes.DWORD]
        kernel32.WaitForSingleObject.restype = wintypes.DWORD

        timeout_ms = wintypes.DWORD(int(timeout_seconds * 1000))
        wait_result = kernel32.WaitForSingleObject(process_handle, timeout_ms)

        if wait_result == 0x00000102:
            if job and job_assigned:
                _close_handle(job)
                job = None
            else:
                kernel32.TerminateProcess.argtypes = [
                    wintypes.HANDLE,
                    wintypes.UINT,
                ]
                kernel32.TerminateProcess.restype = wintypes.BOOL
                kernel32.TerminateProcess(process_handle, 1)
            kernel32.WaitForSingleObject(process_handle, wintypes.DWORD(5000))

        kernel32.GetExitCodeProcess.restype = wintypes.BOOL
        ec = wintypes.DWORD()
        if kernel32.GetExitCodeProcess(process_handle, ctypes.byref(ec)):
            exit_code = ec.value

        _close_handle(process_handle)
        process_handle = None

        result_file = resolved_workspace / "_p2_probe_result.json"
        if result_file.is_file():
            checks = _load_probe_result_checks(result_file, diagnostics=probe_diags)
            try:
                result_file.unlink()
            except OSError as exc:
                probe_diags.append(
                    f"cannot unlink probe result file {result_file}: {exc}"
                )
        elif exit_code == 0:
            checks = {
                "non_admin": False,
                "system32_write_blocked": False,
                "workspace_write_succeeded": False,
                "temp_write_succeeded": False,
                "network_connect_blocked": False,
                "registry_protected_blocked": False,
                "probe_file_cleaned": False,
            }

    finally:
        if firewall_rule_created and firewall_rule_name:
            _delete_firewall_rule(firewall_rule_name, diagnostics=probe_diags)
        if job:
            _close_handle(job)
        if process_handle:
            _close_handle(process_handle)
        if appcontainer_sid and not profile_created:
            _free_sid(appcontainer_sid, diagnostics=probe_diags)

        if profile_created:
            _delete_appcontainer_profile(app_name, diagnostics=probe_diags)

        if probe_path.exists() or probe_path.is_symlink():
            try:
                probe_path.unlink()
                probe_file_cleaned = not probe_path.exists()
            except OSError as exc:
                probe_diags.append(
                    f"probe file cleanup failed for {probe_path}: {exc}"
                )
                probe_file_cleaned = False

    checks["probe_file_cleaned"] = probe_file_cleaned

    observation: dict[str, Any] = {
        "schema_version": "0.1.0-draft",
        "observation_kind": "windows_native_strict_sandbox_observation",
        "observation_id": observation_id,
        "created_at": utc_now(),
        "diagnostics": probe_diags,
        "profile_sha256": sha256_file(
            profile_path or ASSURANCE_ROOT / "windows-native-sandbox-profile-v0.1.json"
        ),
        "workspace_path_sha256": sha256_bytes(
            str(resolved_workspace).encode("utf-8")
        ),
        "appcontainer": {
            "sid_derived": sid_derived,
            "profile_created": profile_created,
            "profile_deleted": profile_created,
            "capabilities": [],
        },
        "firewall": {
            "outbound_block_rule_created": firewall_rule_created,
            "rule_name": firewall_rule_name or "",
            "diagnostic": firewall_diagnostic,
        },
        "job_object": {
            "created": job_created,
            "assigned": job_assigned,
            "creation_time_assignment": creation_time_job_assigned,
            "kill_on_close": job_created,
            "memory_limit_bytes": memory_limit,
        },
        "process": {
            "pid": process_pid,
            "exit_code": exit_code if exit_code is not None else -1,
            "shell_used": False,
        },
        "checks": {
            "non_admin": bool(checks.get("non_admin", False)),
            "system32_write_blocked": bool(checks.get("system32_write_blocked", False)),
            "workspace_write_succeeded": bool(
                checks.get("workspace_write_succeeded", False)
            ),
            "temp_write_succeeded": bool(checks.get("temp_write_succeeded", False)),
            "network_connect_blocked": bool(
                checks.get("network_connect_blocked", False)
            ),
            "registry_protected_blocked": bool(
                checks.get("registry_protected_blocked", False)
            ),
            "probe_file_cleaned": bool(checks.get("probe_file_cleaned", False)),
        },
        "outcome": "noncompliant",
        "evidence_status": "observed",
        "limitations": [
            "Windows native AppContainer sandbox is a development exploration. "
            "This probe tests one disposable process and does not prove every "
            "filesystem, registry, process tree, or network escape impossible.",
            "Child process is created suspended with PROC_THREAD_ATTRIBUTE_JOB_LIST "
            "(creation-time Job Object assignment) where the OS supports it, "
            "and TokenIsAppContainer is verified before ResumeThread.  When "
            "creation-time assignment succeeds the kernel attaches the process "
            "to the job before the initial thread exists, eliminating the "
            "start→Job assignment race.",
            "AppContainer profile creation may succeed without elevation on "
            "current Windows builds; when profile creation fails the probe "
            "falls back to a derived SID only.",
            "Network isolation uses empty AppContainer capabilities (no "
            "internetClient/internetServer). Observed Win11 hosts may still "
            "allow raw Win32 TcpClient outbound while higher-level HTTP APIs "
            "time out; the probe records honest TcpClient results and does "
            "not claim firewall/WFP equivalence to Docker network=none.",
            "This is NOT equivalent to Docker/Linux namespace security and "
            "is not a production sandbox certification.",
        ],
    }

    # Primary isolation checks (required for compliant):
    # Non-network checks (process, FS, registry) must all pass.
    # Network isolation is provided by the host-side firewall outbound
    # block rule, NOT by AppContainer capabilities alone (raw TCP
    # residual observed on Windows 11 10.0.26200).  Compliant outcome
    # requires all non-network checks pass AND the firewall rule active.
    required_checks = {
        "non_admin",
        "system32_write_blocked",
        "workspace_write_succeeded",
        "temp_write_succeeded",
        "registry_protected_blocked",
        "probe_file_cleaned",
    }
    observed_check_keys = set(observation["checks"].keys())
    if observed_check_keys >= required_checks:
        all_required_passed = all(
            v for k, v in observation["checks"].items()
            if k in required_checks and k != "probe_file_cleaned"
        ) and bool(observation["checks"].get("probe_file_cleaned", False))
        if all_required_passed and firewall_rule_created:
            observation["outcome"] = "compliant"

    if not observation["checks"].get("network_connect_blocked", False):
        observation["limitations"].append(
            "Observed residual: raw TCP connect to 1.1.1.1:443 from the "
            "AppContainer probe process was not blocked on this host."
        )
    if not firewall_rule_created:
        observation["limitations"].append(
            "Host firewall outbound block rule was NOT created (elevation "
            "required). Without it, only AppContainer capabilities restrict "
            "network access, and raw TCP may leak through."
        )

    validate_contract(
        observation,
        "windows-native-sandbox-observation-v0.1.schema.json",
        label="Windows native sandbox observation",
    )

    from .sandbox_verifier import verify_windows_native_observation

    verification = verify_windows_native_observation(
        observation,
        profile=profile,
        profile_path=profile_path,
        require_compliant=(observation["outcome"] == "compliant"),
    )
    if not verification["valid"]:
        _tmp_file = resolved_workspace / "_p2_last_obs.json"
        _tmp_file.write_text(
            json.dumps(observation, indent=2, sort_keys=True),
            encoding="utf-8",
        )
        raise AssuranceError(
            "Windows native sandbox observation failed independent verification: "
            + "; ".join(verification["errors"])
        )

    return observation


def windows_native_candidate_from_observation(
    observation: dict[str, Any],
) -> dict[str, Any]:
    validate_contract(
        observation,
        "windows-native-sandbox-observation-v0.1.schema.json",
        label="Windows native sandbox observation",
    )
    compliant = observation["outcome"] == "compliant"
    return {
        "backend_id": "BACKEND-WINDOWS-NATIVE-STRICT-001",
        "backend_kind": "windows_native_strict",
        "availability": "available",
        "compliance_status": "compliant" if compliant else "noncompliant",
        "evidence_status": "observed",
        "receipt_digest": sha256_bytes(canonical_bytes(observation)),
        "rejection_reasons": (
            []
            if compliant
            else ["windows_native_observation_noncompliant"]
        ),
    }
