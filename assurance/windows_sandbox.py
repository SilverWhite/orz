from __future__ import annotations

import base64
import ctypes
from ctypes import wintypes
import json
import os
from pathlib import Path
import subprocess
import threading
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
JOB_OBJECT_LIMIT_ACTIVE_PROCESS = 0x00000008
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
CREATE_NEW_PROCESS_GROUP_FLAG = 0x00000200
CREATE_UNICODE_ENVIRONMENT_FLAG = 0x00000400
TOKEN_QUERY = 0x0008
TokenIsAppContainer = 29
CTRL_BREAK_EVENT = 1
# GAK-06: grace period between CTRL_BREAK and job close (seconds)
CANCEL_GRACE_SECONDS = 5


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


def _resolve_windows_powershell_exe() -> str:
    system_root = Path(os.environ.get("SystemRoot", r"C:\Windows"))
    candidates = [
        system_root / "System32" / "WindowsPowerShell" / "v1.0" / "powershell.exe",
        system_root / "Sysnative" / "WindowsPowerShell" / "v1.0" / "powershell.exe",
    ]
    for candidate in candidates:
        if candidate.is_file():
            return str(candidate)
    raise AssuranceError(
        "Windows native sandbox probe could not locate Windows PowerShell"
    )


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
            wintypes.DWORD,
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


def _lookup_account_sid_string(account: str) -> str | None:
    """Resolve a local account (user or domain\\user) to its SID string."""
    if os.name != "nt":
        return None
    try:
        advapi32 = ctypes.WinDLL("advapi32", use_last_error=True)
        advapi32.LookupAccountNameW.argtypes = [
            wintypes.LPCWSTR,
            wintypes.LPCWSTR,
            ctypes.c_void_p,
            ctypes.POINTER(wintypes.DWORD),
            wintypes.LPWSTR,
            ctypes.POINTER(wintypes.DWORD),
            ctypes.POINTER(wintypes.DWORD),
        ]
        advapi32.LookupAccountNameW.restype = wintypes.BOOL
        size = wintypes.DWORD()
        domain = ctypes.create_unicode_buffer(256)
        domain_size = wintypes.DWORD(len(domain))
        use = wintypes.DWORD()
        advapi32.LookupAccountNameW(
            None, account, None, ctypes.byref(size), domain,
            ctypes.byref(domain_size), ctypes.byref(use),
        )
        sid_buf = ctypes.create_string_buffer(max(int(size.value), 8))
        if not advapi32.LookupAccountNameW(
            None, account, ctypes.cast(sid_buf, ctypes.c_void_p),
            ctypes.byref(size), domain, ctypes.byref(domain_size),
            ctypes.byref(use),
        ):
            return None
        sid_str_ptr = wintypes.LPWSTR()
        if not advapi32.ConvertSidToStringSidW(
            ctypes.cast(sid_buf, ctypes.c_void_p), ctypes.byref(sid_str_ptr)
        ):
            return None
        result = sid_str_ptr.value
        kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
        kernel32.LocalFree.argtypes = [ctypes.c_void_p]
        kernel32.LocalFree.restype = ctypes.c_void_p
        kernel32.LocalFree(sid_str_ptr)
        return result
    except OSError:
        return None


def _grant_session_desktop_access(
    account: str,
    *,
    extra_sids: list[str] | None = None,
    diagnostics: list[str] | None = None,
) -> bool:
    """Grant GENERIC_ALL on the current session window station and desktop
    to a local account.

    S4 finding: a process spawned with another user's token from a
    scheduled-task session (session 0) fails DLL initialization
    (STATUS_DLL_INIT_FAILED / 0xC0000142) because the service window
    station/desktop ACL only grants the task owner + Administrators.  The
    child inherits the caller's window station/desktop, so the run user
    needs explicit access there.  The service window station is recreated
    on every boot, so the grant must be (re)applied at spawn time.
    Idempotent: skips SIDs already present in the DACL.
    """
    if os.name != "nt":
        return False
    sid_string = _lookup_account_sid_string(account)
    if not sid_string:
        if diagnostics is not None:
            diagnostics.append(
                f"desktop grant: cannot resolve SID for {account}"
            )
        return False
    grant_sids = [sid_string] + list(extra_sids or [])
    try:
        user32 = ctypes.WinDLL("user32", use_last_error=True)
        advapi32 = ctypes.WinDLL("advapi32", use_last_error=True)
        kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)

        user32.GetProcessWindowStation.argtypes = []
        user32.GetProcessWindowStation.restype = wintypes.HANDLE
        hwinsta = user32.GetProcessWindowStation()
        kernel32.GetCurrentThreadId.argtypes = []
        kernel32.GetCurrentThreadId.restype = wintypes.DWORD
        user32.GetThreadDesktop.argtypes = [wintypes.DWORD]
        user32.GetThreadDesktop.restype = wintypes.HANDLE
        hdesk = user32.GetThreadDesktop(kernel32.GetCurrentThreadId())

        all_si = wintypes.DWORD(0x7)  # OWNER | GROUP | DACL
        user32.GetUserObjectSecurity.argtypes = [
            wintypes.HANDLE,
            ctypes.POINTER(wintypes.DWORD),
            ctypes.c_void_p,
            wintypes.DWORD,
            ctypes.POINTER(wintypes.DWORD),
        ]
        user32.GetUserObjectSecurity.restype = wintypes.BOOL
        user32.SetUserObjectSecurity.argtypes = [
            wintypes.HANDLE,
            ctypes.POINTER(wintypes.DWORD),
            ctypes.c_void_p,
        ]
        user32.SetUserObjectSecurity.restype = wintypes.BOOL
        advapi32.ConvertSecurityDescriptorToStringSecurityDescriptorW.argtypes = [
            ctypes.c_void_p,
            wintypes.DWORD,
            wintypes.DWORD,
            ctypes.POINTER(ctypes.c_wchar_p),
        ]
        advapi32.ConvertSecurityDescriptorToStringSecurityDescriptorW.restype = (
            wintypes.BOOL
        )
        advapi32.ConvertStringSecurityDescriptorToSecurityDescriptorW.argtypes = [
            wintypes.LPCWSTR,
            wintypes.DWORD,
            ctypes.POINTER(ctypes.c_void_p),
            ctypes.POINTER(ctypes.c_ulong),
        ]
        advapi32.ConvertStringSecurityDescriptorToSecurityDescriptorW.restype = (
            wintypes.BOOL
        )
        kernel32.LocalFree.argtypes = [ctypes.c_void_p]
        kernel32.LocalFree.restype = ctypes.c_void_p

        def _read_sddl(h: wintypes.HANDLE) -> str | None:
            need = wintypes.DWORD()
            small = ctypes.create_string_buffer(1)
            user32.GetUserObjectSecurity(
                h, ctypes.byref(all_si), ctypes.cast(small, ctypes.c_void_p),
                0, ctypes.byref(need),
            )
            buf = ctypes.create_string_buffer(max(int(need.value), 1))
            if not user32.GetUserObjectSecurity(
                h, ctypes.byref(all_si), ctypes.cast(buf, ctypes.c_void_p),
                ctypes.sizeof(buf), ctypes.byref(need),
            ):
                return None
            sddl = ctypes.c_wchar_p()
            if not advapi32.ConvertSecurityDescriptorToStringSecurityDescriptorW(
                ctypes.cast(buf, ctypes.c_void_p), 1, 0x7, ctypes.byref(sddl)
            ):
                return None
            result = sddl.value
            kernel32.LocalFree(sddl)
            return result

        def _grant(h: wintypes.HANDLE, sddl: str | None) -> bool:
            if sddl is None:
                return sddl is not None
            for sid in grant_sids:
                if sid in sddl:
                    continue
                i = sddl.find("D:(")
                if i < 0:
                    return False
                i += 2
                new_sddl = sddl[:i] + f"(A;;GA;;;{sid})" + sddl[i:]
                sd = ctypes.c_void_p()
                sz = ctypes.c_ulong()
                if not advapi32.ConvertStringSecurityDescriptorToSecurityDescriptorW(
                    new_sddl, 1, ctypes.byref(sd), ctypes.byref(sz)
                ):
                    return False
                ok = user32.SetUserObjectSecurity(
                    h, ctypes.byref(all_si), sd.value
                )
                kernel32.LocalFree(sd)
                if not ok:
                    return False
                sddl = new_sddl
            return True

        ok_ws = _grant(hwinsta, _read_sddl(hwinsta))
        ok_ds = _grant(hdesk, _read_sddl(hdesk))
        if not (ok_ws and ok_ds):
            if diagnostics is not None:
                diagnostics.append(
                    "desktop grant: window station/desktop ACL update failed "
                    f"(winsta={ok_ws}, desktop={ok_ds})"
                )
        return ok_ws and ok_ds
    except OSError as exc:
        if diagnostics is not None:
            diagnostics.append(f"desktop grant exception: {exc}")
        return False


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


def _create_kill_on_close_job(
    memory_limit_bytes: int,
    active_process_limit: int | None = None,
) -> wintypes.HANDLE | None:
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
    if active_process_limit is not None:
        limits.BasicLimitInformation.LimitFlags |= JOB_OBJECT_LIMIT_ACTIVE_PROCESS
        limits.BasicLimitInformation.ActiveProcessLimit = int(active_process_limit)
    succeeded = kernel32.SetInformationJobObject(
        _as_handle(handle),
        JOB_OBJECT_EXTENDED_LIMIT_INFORMATION_CLASS,
        ctypes.byref(limits),
        ctypes.sizeof(limits),
    )
    if not succeeded:
        kernel32.CloseHandle(_as_handle(handle))
        return None
    return _as_handle(handle)


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

    ps_exe = _resolve_windows_powershell_exe()

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
        # GAK-06: CREATE_NEW_PROCESS_GROUP enables CTRL_BREAK_EVENT
        # delivery to the process group on timeout.
        creation_flags = (
            EXTENDED_STARTUPINFO_PRESENT_FLAG
            | CREATE_SUSPENDED_FLAG
            | CREATE_NO_WINDOW_FLAG
            | CREATE_NEW_PROCESS_GROUP_FLAG
        )
        success = kernel32.CreateProcessW(
            ps_exe,
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

        # GAK-06: staged cancellation per WIN-PROC-003
        cancellation_method: str | None = None
        ctrl_break_sent = False
        ctrl_break_effective = False

        if wait_result == 0x00000102:
            # Stage 1: send CTRL_BREAK_EVENT to the process group
            cancellation_method = "timeout"
            kernel32.GenerateConsoleCtrlEvent.argtypes = [
                wintypes.DWORD,
                wintypes.DWORD,
            ]
            kernel32.GenerateConsoleCtrlEvent.restype = wintypes.BOOL
            ctrl_break_sent = kernel32.GenerateConsoleCtrlEvent(
                wintypes.DWORD(CTRL_BREAK_EVENT),
                wintypes.DWORD(process_pid),
            )
            probe_diags.append(
                f"CTRL_BREAK_EVENT sent={ctrl_break_sent} to pid={process_pid}"
            )

            # Stage 2: wait for the grace period
            if ctrl_break_sent:
                grace_ms = wintypes.DWORD(int(CANCEL_GRACE_SECONDS * 1000))
                grace_result = kernel32.WaitForSingleObject(
                    process_handle, grace_ms,
                )
                ctrl_break_effective = grace_result == 0  # WAIT_OBJECT_0
                if ctrl_break_effective:
                    cancellation_method = "ctrl_break"
                    probe_diags.append(
                        "CTRL_BREAK_EVENT effective — process exited during grace"
                    )
                else:
                    probe_diags.append(
                        "CTRL_BREAK_EVENT did not terminate process within grace"
                    )

            # Stage 3: if still running, use Job Object kill-on-close
            # as the primary containment termination
            if not ctrl_break_effective:
                if job and job_assigned:
                    cancellation_method = "job_close"
                    _close_handle(job)
                    job = None
                    probe_diags.append(
                        "job closed (kill-on-close) after CTRL_BREAK"
                    )
                else:
                    # Stage 4: fallback — direct TerminateProcess
                    cancellation_method = "terminate_process"
                    kernel32.TerminateProcess.argtypes = [
                        wintypes.HANDLE,
                        wintypes.UINT,
                    ]
                    kernel32.TerminateProcess.restype = wintypes.BOOL
                    kernel32.TerminateProcess(process_handle, 1)
                    probe_diags.append(
                        "TerminateProcess used — Job Object unavailable"
                    )

            # Final drain: wait for process exit
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
            "create_new_process_group": True,
        },
        "cancellation": {
            "timeout_seconds": timeout_seconds,
            "cancellation_method": cancellation_method,
            "ctrl_break_sent": ctrl_break_sent,
            "ctrl_break_effective": ctrl_break_effective,
            "grace_period_seconds": CANCEL_GRACE_SECONDS if wait_result == 0x00000102 else 0,
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


# ---------------------------------------------------------------------------
# P0-0l ② Windows 加固运行环境（restricted token / LOW IL / AppContainer /
# Job Object 下 spawn 任意命令树）。探针（run_windows_native_sandbox_probe）
# 保持原样；这里新增的是“运行环境”入口，供 enforcement-probe 与 orz 命令树
# 在加固态下被拉起。所有 spawn 期断言（token/完整性/AppContainer/Job/临时
# 目录重定向）进入 run observation；行为级墙断言（System32/Program Files/
# HKLM/home/网络等）由 enforcement_probe.ps1 在墙内执行。
# ---------------------------------------------------------------------------

WINDOWS_RUN_ARMS = ("control", "non-admin", "high-nist")

# Run-user logon mode: when ORZ_WINDOWS_RUN_USER is set, the non-admin and
# high-nist arms spawn the child under a REAL standard-user logon token
# (RunUser from apply_hardening.ps1) instead of a CreateRestrictedToken
# restricted token.  Restricted tokens that disable BUILTIN\Administrators
# cannot start any process on Windows 11 25H2 (STATUS_DLL_INIT_FAILED /
# 0xC0000142 observed in S4), and the hardened ACLs are scoped to RunUser,
# so the logon-token mode is both correct and necessary.
WINDOWS_RUN_USER_ENV = "ORZ_WINDOWS_RUN_USER"
WINDOWS_RUN_USER_PASSWORD_ENV = "ORZ_WINDOWS_RUN_USER_PASSWORD"
WINDOWS_RUN_USER_DOMAIN_ENV = "ORZ_WINDOWS_RUN_USER_DOMAIN"
LOGON32_LOGON_BATCH = 4
LOGON32_PROVIDER_DEFAULT = 0
SE_PRIVILEGE_ENABLED = 0x00000002
TOKEN_ADJUST_PRIVILEGES = 0x0020
TOKEN_QUERY = 0x0008

# Privilege·non-admin：受限 token 移除的特权（设计 §3）。
WINDOWS_RUN_RESTRICTED_PRIVILEGES = (
    "SeDebugPrivilege",
    "SeBackupPrivilege",
    "SeRestorePrivilege",
    "SeTakeOwnershipPrivilege",
    "SeLoadDriverPrivilege",
    "SeCreateSymbolicLinkPrivilege",
)

# 受限 token：禁用/deny-only BUILTIN\Administrators（S-1-5-32-544）。
WINDOWS_RUN_DISABLE_SIDS = ("S-1-5-32-544",)
WINDOWS_RUN_DENY_ONLY_SIDS = ("S-1-5-32-544",)

TOKEN_DUPLICATE = 0x0002
TOKEN_ASSIGN_PRIMARY = 0x0001
TOKEN_ADJUST_DEFAULT = 0x0080
TOKEN_ALL_ACCESS = 0x000F01FF

TokenVirtualizationAllowed = 24
TokenIntegrityLevel = 25
TokenGroups = 2
TokenPrivileges = 3

SE_GROUP_ENABLED = 0x00000004
SE_GROUP_USE_FOR_DENY_ONLY = 0x00000010
SE_GROUP_INTEGRITY = 0x00000020
SecurityImpersonation = 2
TokenPrimary = 1
SECURITY_MANDATORY_LOW_RID = 0x1000  # 4096 = Low integrity

STARTF_USESTDHANDLES = 0x00000100
HANDLE_FLAG_INHERIT = 0x00000001

# 每臂 run observation 的 checks 集合（spawn 期 + harness 侧事实）。
# 行为级断言见 _windows_high_nist/policy/enforcement_probe.ps1。
WINDOWS_RUN_OBSERVATION_CHECKS: dict[str, tuple[str, ...]] = {
    "control": ("workspace_writable",),
    "non-admin": (
        "workspace_writable",
        "non_admin",
        "token_virtualization_disabled",
        "privileges_removed",
    ),
    "high-nist": (
        "workspace_writable",
        "non_admin",
        "token_virtualization_disabled",
        "privileges_removed",
        "low_integrity",
        "appcontainer_token",
        "temp_redirected",
        "job_object_assigned",
    ),
}


def build_restricted_token_spec(arm: str) -> dict[str, Any]:
    """Return the restricted-token spec applied for the arm (pure, testable)."""
    if arm == "control":
        return {
            "disable_sids": [],
            "deny_only_sids": [],
            "remove_privileges": [],
            "low_integrity": False,
            "appcontainer": False,
            "virtualization_allowed": True,
        }
    if arm not in WINDOWS_RUN_ARMS:
        raise AssuranceError(f"unknown arm {arm!r} (expected {WINDOWS_RUN_ARMS})")
    spec: dict[str, Any] = {
        "disable_sids": list(WINDOWS_RUN_DISABLE_SIDS),
        "deny_only_sids": list(WINDOWS_RUN_DENY_ONLY_SIDS),
        "remove_privileges": list(WINDOWS_RUN_RESTRICTED_PRIVILEGES),
        "low_integrity": arm == "high-nist",
        "appcontainer": arm == "high-nist",
        "virtualization_allowed": False,
    }
    return spec


def run_observation_checks_for_arm(arm: str) -> tuple[str, ...]:
    if arm not in WINDOWS_RUN_ARMS:
        raise AssuranceError(f"unknown arm {arm!r} (expected {WINDOWS_RUN_ARMS})")
    return WINDOWS_RUN_OBSERVATION_CHECKS[arm]


class _LUID(ctypes.Structure):
    _fields_ = [
        ("LowPart", wintypes.DWORD),
        ("HighPart", wintypes.LONG),
    ]


class _LUID_AND_ATTRIBUTES(ctypes.Structure):
    _fields_ = [
        ("Luid", _LUID),
        ("Attributes", wintypes.DWORD),
    ]


class _SID_AND_ATTRIBUTES(ctypes.Structure):
    _fields_ = [
        ("Sid", ctypes.c_void_p),
        ("Attributes", wintypes.DWORD),
    ]


class _TOKEN_MANDATORY_LABEL(ctypes.Structure):
    _fields_ = [("Label", _SID_AND_ATTRIBUTES)]


def _as_handle(value) -> wintypes.HANDLE | None:
    """Normalize an int / pointer / HANDLE to a wintypes.HANDLE.

    ctypes rejects double-wrapping an existing c_void_p (e.g.
    wintypes.HANDLE(existing_handle)), so return c_void_p instances as-is.
    """
    if value is None:
        return None
    if isinstance(value, ctypes.c_void_p):
        return value
    return wintypes.HANDLE(value)


def _string_sid_to_ptr(
    sid_string: str, *, diagnostics: list[str] | None = None
) -> ctypes.c_void_p | None:
    if os.name != "nt":
        return None
    try:
        advapi32 = ctypes.WinDLL("advapi32", use_last_error=True)
        advapi32.ConvertStringSidToSidW.argtypes = [
            wintypes.LPCWSTR,
            ctypes.POINTER(ctypes.c_void_p),
        ]
        advapi32.ConvertStringSidToSidW.restype = wintypes.BOOL
        sid_ptr = ctypes.c_void_p()
        if not advapi32.ConvertStringSidToSidW(sid_string, ctypes.byref(sid_ptr)):
            return None
        return sid_ptr
    except OSError as exc:
        if diagnostics is not None:
            diagnostics.append(f"ConvertStringSidToSidW({sid_string}) failed: {exc}")
        return None


def _lookup_privilege_luid(
    name: str, *, diagnostics: list[str] | None = None
) -> _LUID | None:
    if os.name != "nt":
        return None
    try:
        advapi32 = ctypes.WinDLL("advapi32", use_last_error=True)
        advapi32.LookupPrivilegeValueW.argtypes = [
            wintypes.LPCWSTR,
            wintypes.LPCWSTR,
            ctypes.POINTER(_LUID),
        ]
        advapi32.LookupPrivilegeValueW.restype = wintypes.BOOL
        luid = _LUID()
        if not advapi32.LookupPrivilegeValueW(None, name, ctypes.byref(luid)):
            return None
        return luid
    except OSError as exc:
        if diagnostics is not None:
            diagnostics.append(f"LookupPrivilegeValueW({name}) failed: {exc}")
        return None


def _set_token_virtualization(
    token: wintypes.HANDLE,
    enabled: bool,
    *,
    diagnostics: list[str] | None = None,
) -> bool:
    if os.name != "nt" or not token:
        return False
    try:
        advapi32 = ctypes.WinDLL("advapi32", use_last_error=True)
        advapi32.SetTokenInformation.argtypes = [
            wintypes.HANDLE,
            ctypes.c_int,
            ctypes.c_void_p,
            wintypes.DWORD,
        ]
        advapi32.SetTokenInformation.restype = wintypes.BOOL
        value = wintypes.BOOL(bool(enabled))
        return bool(
            advapi32.SetTokenInformation(
                token,
                TokenVirtualizationAllowed,
                ctypes.byref(value),
                ctypes.sizeof(value),
            )
        )
    except OSError as exc:
        if diagnostics is not None:
            diagnostics.append(f"SetTokenInformation(virtualization) failed: {exc}")
        return False


def _set_token_integrity(
    token: wintypes.HANDLE,
    rid: int = SECURITY_MANDATORY_LOW_RID,
    *,
    diagnostics: list[str] | None = None,
) -> bool:
    if os.name != "nt" or not token:
        return False
    sid = ctypes.c_void_p()
    try:
        advapi32 = ctypes.WinDLL("advapi32", use_last_error=True)
        authority = (ctypes.c_ubyte * 6)(0, 0, 0, 0, 0, 16)
        advapi32.AllocateAndInitializeSid.argtypes = [
            ctypes.POINTER(ctypes.c_ubyte * 6),
            wintypes.DWORD,
        ] + [wintypes.DWORD] * 8 + [ctypes.POINTER(ctypes.c_void_p)]
        advapi32.AllocateAndInitializeSid.restype = wintypes.BOOL
        if not advapi32.AllocateAndInitializeSid(
            authority,
            1,
            rid,
            0, 0, 0, 0, 0, 0, 0,
            ctypes.byref(sid),
        ):
            return False
        label = _TOKEN_MANDATORY_LABEL()
        label.Label.Sid = sid
        label.Label.Attributes = SE_GROUP_INTEGRITY
        advapi32.SetTokenInformation.argtypes = [
            wintypes.HANDLE,
            ctypes.c_int,
            ctypes.c_void_p,
            wintypes.DWORD,
        ]
        advapi32.SetTokenInformation.restype = wintypes.BOOL
        return bool(
            advapi32.SetTokenInformation(
                token,
                TokenIntegrityLevel,
                ctypes.byref(label),
                ctypes.sizeof(label),
            )
        )
    except OSError as exc:
        if diagnostics is not None:
            diagnostics.append(f"SetTokenInformation(integrity) failed: {exc}")
        return False
    finally:
        if sid:
            _free_sid(sid, diagnostics=diagnostics)


def _create_restricted_token(
    spec: dict[str, Any], *, diagnostics: list[str] | None = None
) -> wintypes.HANDLE | None:
    """CreateRestrictedToken from the current process token per the arm spec."""
    if os.name != "nt":
        return None
    kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
    advapi32 = ctypes.WinDLL("advapi32", use_last_error=True)
    current = wintypes.HANDLE()
    kernel32.OpenProcessToken.argtypes = [
        wintypes.HANDLE,
        wintypes.DWORD,
        ctypes.POINTER(wintypes.HANDLE),
    ]
    kernel32.OpenProcessToken.restype = wintypes.BOOL
    if not kernel32.OpenProcessToken(
        kernel32.GetCurrentProcess(),
        TOKEN_DUPLICATE | TOKEN_QUERY | TOKEN_ASSIGN_PRIMARY | TOKEN_ADJUST_DEFAULT,
        ctypes.byref(current),
    ):
        if diagnostics is not None:
            diagnostics.append(
                f"OpenProcessToken failed: {ctypes.WinError(ctypes.get_last_error())}"
            )
        return None
    allocated: list[ctypes.c_void_p] = []
    try:
        # SidsToDisable entries: disable with Attributes=0, or deny-only with
        # SE_GROUP_USE_FOR_DENY_ONLY (the classic deny-only mechanism; this
        # does NOT create restricted SIDs, so workspace access is preserved).
        handled_sids: set[str] = set()
        disable_entries: list[_SID_AND_ATTRIBUTES] = []
        for sid_string in list(spec.get("disable_sids", []) or []):
            sid = _string_sid_to_ptr(sid_string, diagnostics=diagnostics)
            if sid:
                entry = _SID_AND_ATTRIBUTES()
                entry.Sid = sid
                # Disabled (Attributes=0), not deny-only: deny-only
                # Administrators is documented to break DLL initialization in
                # restricted tokens (STATUS_DLL_INIT_FAILED observed in S4).
                entry.Attributes = 0
                disable_entries.append(entry)
                allocated.append(sid)
                handled_sids.add(sid_string)
        for sid_string in list(spec.get("deny_only_sids", []) or []):
            if sid_string in handled_sids:
                continue
            sid = _string_sid_to_ptr(sid_string, diagnostics=diagnostics)
            if sid:
                entry = _SID_AND_ATTRIBUTES()
                entry.Sid = sid
                entry.Attributes = SE_GROUP_USE_FOR_DENY_ONLY
                disable_entries.append(entry)
                allocated.append(sid)

        # Compact the LUID list: failed lookups must not leave gaps that
        # CreateRestrictedToken would interpret as a different privilege.
        resolved: list[tuple[str, _LUID]] = []
        for name in list(spec.get("remove_privileges", []) or []):
            luid = _lookup_privilege_luid(name, diagnostics=diagnostics)
            if luid is not None:
                resolved.append((name, luid))

        disable_array = (_SID_AND_ATTRIBUTES * max(len(disable_entries), 1))()
        for index, entry in enumerate(disable_entries):
            disable_array[index] = entry
        priv_array = (_LUID_AND_ATTRIBUTES * max(len(resolved), 1))()
        for index, (_, luid) in enumerate(resolved):
            priv_array[index].Luid = luid
            priv_array[index].Attributes = 0

        new_token = wintypes.HANDLE()
        advapi32.CreateRestrictedToken.argtypes = [
            wintypes.HANDLE,
            wintypes.DWORD,
            wintypes.DWORD,
            ctypes.c_void_p,
            wintypes.DWORD,
            ctypes.c_void_p,
            wintypes.DWORD,
            ctypes.c_void_p,
            ctypes.POINTER(wintypes.HANDLE),
        ]
        advapi32.CreateRestrictedToken.restype = wintypes.BOOL
        ok = advapi32.CreateRestrictedToken(
            current,
            0,
            len(disable_entries),
            disable_array if disable_entries else None,
            len(resolved),
            priv_array if resolved else None,
            0,
            None,
            ctypes.byref(new_token),
        )
        if not ok:
            if diagnostics is not None:
                diagnostics.append(
                    "CreateRestrictedToken failed: "
                    f"{ctypes.WinError(ctypes.get_last_error())}"
                )
            return None
        if not spec.get("virtualization_allowed", True):
            if not _set_token_virtualization(
                new_token, False, diagnostics=diagnostics
            ):
                if diagnostics is not None:
                    diagnostics.append(
                        "TokenVirtualizationAllowed=0 could not be set on "
                        "restricted token (fail-closed at check time)"
                    )
        return _as_handle(new_token)
    finally:
        for sid in allocated:
            _free_sid(sid, diagnostics=diagnostics)
        kernel32.CloseHandle(current)


def _duplicate_token_primary(
    token: wintypes.HANDLE, *, diagnostics: list[str] | None = None
) -> wintypes.HANDLE | None:
    if os.name != "nt" or not token:
        return None
    try:
        advapi32 = ctypes.WinDLL("advapi32", use_last_error=True)
        advapi32.DuplicateTokenEx.argtypes = [
            wintypes.HANDLE,
            wintypes.DWORD,
            ctypes.c_void_p,
            ctypes.c_int,
            ctypes.c_int,
            ctypes.POINTER(wintypes.HANDLE),
        ]
        advapi32.DuplicateTokenEx.restype = wintypes.BOOL
        primary = wintypes.HANDLE()
        if not advapi32.DuplicateTokenEx(
            token,
            TOKEN_ALL_ACCESS,
            None,
            SecurityImpersonation,
            TokenPrimary,
            ctypes.byref(primary),
        ):
            if diagnostics is not None:
                diagnostics.append(
                    "DuplicateTokenEx(primary) failed: "
                    f"{ctypes.WinError(ctypes.get_last_error())}"
                )
            return None
        return _as_handle(primary)
    except OSError as exc:
        if diagnostics is not None:
            diagnostics.append(f"DuplicateTokenEx exception: {exc}")
        return None


def _get_token_information(
    token: wintypes.HANDLE,
    info_class: int,
    size: int,
    *,
    diagnostics: list[str] | None = None,
) -> bytes | None:
    if os.name != "nt" or not token:
        return None
    try:
        advapi32 = ctypes.WinDLL("advapi32", use_last_error=True)
        advapi32.GetTokenInformation.argtypes = [
            wintypes.HANDLE,
            ctypes.c_int,
            ctypes.c_void_p,
            wintypes.DWORD,
            ctypes.POINTER(wintypes.DWORD),
        ]
        advapi32.GetTokenInformation.restype = wintypes.BOOL
        needed = wintypes.DWORD()
        ok = advapi32.GetTokenInformation(
            token, info_class, None, 0, ctypes.byref(needed)
        )
        if not ok and ctypes.get_last_error() != 122:  # ERROR_INSUFFICIENT_BUFFER
            return None
        buf = ctypes.create_string_buffer(max(needed.value, size))
        ret_len = wintypes.DWORD()
        if not advapi32.GetTokenInformation(
            token,
            info_class,
            ctypes.cast(buf, ctypes.c_void_p),
            len(buf),
            ctypes.byref(ret_len),
        ):
            return None
        return buf.raw[: ret_len.value]
    except OSError as exc:
        if diagnostics is not None:
            diagnostics.append(
                f"GetTokenInformation(class={info_class}) failed: {exc}"
            )
        return None


def _token_bool(
    token: wintypes.HANDLE,
    info_class: int,
    *,
    diagnostics: list[str] | None = None,
) -> bool | None:
    raw = _get_token_information(token, info_class, 4, diagnostics=diagnostics)
    if raw is None or len(raw) < 4:
        return None
    return bool(int.from_bytes(raw[:4], "little"))


def _token_integrity_rid(
    token: wintypes.HANDLE, *, diagnostics: list[str] | None = None
) -> int | None:
    raw = _get_token_information(
        token, TokenIntegrityLevel, ctypes.sizeof(_TOKEN_MANDATORY_LABEL),
        diagnostics=diagnostics,
    )
    if raw is None or len(raw) < ctypes.sizeof(_TOKEN_MANDATORY_LABEL):
        return None
    label = _TOKEN_MANDATORY_LABEL.from_buffer_copy(
        raw[: ctypes.sizeof(_TOKEN_MANDATORY_LABEL)]
    )
    sid = label.Label.Sid
    if not sid:
        return None
    try:
        advapi32 = ctypes.WinDLL("advapi32", use_last_error=True)
        advapi32.GetSidSubAuthorityCount.argtypes = [ctypes.c_void_p]
        advapi32.GetSidSubAuthorityCount.restype = ctypes.POINTER(ctypes.c_ubyte)
        advapi32.GetSidSubAuthority.argtypes = [ctypes.c_void_p, wintypes.DWORD]
        advapi32.GetSidSubAuthority.restype = ctypes.POINTER(wintypes.DWORD)
        count_ptr = advapi32.GetSidSubAuthorityCount(sid)
        if not count_ptr:
            return None
        count = count_ptr.contents.value
        if count < 1:
            return None
        sub = advapi32.GetSidSubAuthority(sid, count - 1)
        if not sub:
            return None
        return int(sub.contents.value)
    except OSError as exc:
        if diagnostics is not None:
            diagnostics.append(f"integrity RID query failed: {exc}")
        return None


def _token_has_enabled_group(
    token: wintypes.HANDLE,
    sid_string: str,
    *,
    diagnostics: list[str] | None = None,
) -> bool | None:
    raw = _get_token_information(token, TokenGroups, 0, diagnostics=diagnostics)
    if raw is None or len(raw) < 4:
        return None
    count = int.from_bytes(raw[:4], "little")
    pointer_size = ctypes.sizeof(ctypes.c_void_p)
    groups_offset = 8 if pointer_size == 8 else 4
    entry_size = ctypes.sizeof(_SID_AND_ATTRIBUTES)
    for index in range(count):
        start = groups_offset + index * entry_size
        if start + entry_size > len(raw):
            break
        sid_ptr = int.from_bytes(raw[start : start + pointer_size], "little")
        attributes = int.from_bytes(
            raw[start + pointer_size : start + pointer_size + 4], "little"
        )
        if not sid_ptr:
            continue
        sid = ctypes.c_void_p(sid_ptr)
        current = _appcontainer_sid_to_string(sid)
        if current == sid_string and (attributes & SE_GROUP_ENABLED):
            return True
    return False


def _token_enabled_privileges(
    token: wintypes.HANDLE, *, diagnostics: list[str] | None = None
) -> list[str]:
    raw = _get_token_information(token, TokenPrivileges, 0, diagnostics=diagnostics)
    if raw is None or len(raw) < 4:
        return []
    count = int.from_bytes(raw[:4], "little")
    entry_size = ctypes.sizeof(_LUID_AND_ATTRIBUTES)
    privileges_offset = 4
    names: list[str] = []
    for index in range(count):
        start = privileges_offset + index * entry_size
        if start + entry_size > len(raw):
            break
        luid = _LUID()
        luid.LowPart = int.from_bytes(raw[start : start + 4], "little")
        luid.HighPart = int.from_bytes(
            raw[start + 4 : start + 8], "little", signed=True
        )
        attributes = int.from_bytes(raw[start + 8 : start + 12], "little")
        if not (attributes & SE_GROUP_ENABLED):
            continue
        name = _privilege_name_from_luid(luid, diagnostics=diagnostics)
        if name:
            names.append(name)
    return names


def _privilege_name_from_luid(
    luid: _LUID, *, diagnostics: list[str] | None = None
) -> str | None:
    if os.name != "nt":
        return None
    try:
        advapi32 = ctypes.WinDLL("advapi32", use_last_error=True)
        advapi32.LookupPrivilegeNameW.argtypes = [
            wintypes.LPCWSTR,
            ctypes.POINTER(_LUID),
            wintypes.LPWSTR,
            ctypes.POINTER(wintypes.DWORD),
        ]
        advapi32.LookupPrivilegeNameW.restype = wintypes.BOOL
        size = wintypes.DWORD(0)
        advapi32.LookupPrivilegeNameW(None, ctypes.byref(luid), None, ctypes.byref(size))
        if size.value == 0:
            return None
        buf = ctypes.create_unicode_buffer(size.value)
        if not advapi32.LookupPrivilegeNameW(
            None, ctypes.byref(luid), buf, ctypes.byref(size)
        ):
            return None
        return buf.value
    except OSError as exc:
        if diagnostics is not None:
            diagnostics.append(f"LookupPrivilegeNameW failed: {exc}")
        return None


def _grant_workspace_access_current_user(
    workspace: Path, *, diagnostics: list[str] | None = None
) -> bool:
    if os.name != "nt":
        return False
    username = os.environ.get("USERNAME", "")
    userdomain = os.environ.get("USERDOMAIN", "")
    if not username:
        return False
    account = f"{userdomain}\\{username}" if userdomain else username
    try:
        result = subprocess.run(
            ["icacls", str(workspace), "/grant", f"{account}:(OI)(CI)(M)"],
            capture_output=True,
            shell=False,
            timeout=15,
        )
        return result.returncode == 0
    except Exception as exc:
        if diagnostics is not None:
            diagnostics.append(
                f"icacls grant for {workspace} to {account} failed: {exc}"
            )
        return False


def _grant_workspace_access_account(
    workspace: Path,
    domain: str,
    user: str,
    *,
    diagnostics: list[str] | None = None,
) -> bool:
    if os.name != "nt":
        return False
    account = f"{domain}\\{user}" if domain else user
    try:
        result = subprocess.run(
            ["icacls", str(workspace), "/grant", f"{account}:(OI)(CI)(M)"],
            capture_output=True,
            shell=False,
            timeout=15,
        )
        return result.returncode == 0
    except Exception as exc:
        if diagnostics is not None:
            diagnostics.append(
                f"icacls grant for {workspace} to {account} failed: {exc}"
            )
        return False


def _get_run_user_credentials() -> tuple[str, str, str] | None:
    """Return (domain, user, password) for the non-admin run-user logon."""
    user = os.environ.get(WINDOWS_RUN_USER_ENV, "")
    if not user:
        return None
    password = os.environ.get(WINDOWS_RUN_USER_PASSWORD_ENV, "")
    domain = os.environ.get(
        WINDOWS_RUN_USER_DOMAIN_ENV, os.environ.get("COMPUTERNAME", "")
    )
    return (domain, user, password)


def _logon_run_user(
    credentials: tuple[str, str, str],
    *,
    diagnostics: list[str] | None = None,
) -> wintypes.HANDLE | None:
    if os.name != "nt":
        return None
    domain, user, password = credentials
    try:
        advapi32 = ctypes.WinDLL("advapi32", use_last_error=True)
        advapi32.LogonUserW.argtypes = [
            wintypes.LPCWSTR,
            wintypes.LPCWSTR,
            wintypes.LPCWSTR,
            wintypes.DWORD,
            wintypes.DWORD,
            ctypes.POINTER(wintypes.HANDLE),
        ]
        advapi32.LogonUserW.restype = wintypes.BOOL
        token = wintypes.HANDLE()
        if not advapi32.LogonUserW(
            user,
            domain,
            password,
            LOGON32_LOGON_BATCH,
            LOGON32_PROVIDER_DEFAULT,
            ctypes.byref(token),
        ):
            if diagnostics is not None:
                diagnostics.append(
                    f"LogonUserW({domain}\\{user}) failed: "
                    f"{ctypes.WinError(ctypes.get_last_error())}"
                )
            return None
        return token
    except OSError as exc:
        if diagnostics is not None:
            diagnostics.append(f"LogonUserW exception: {exc}")
        return None


def _load_run_user_profile(
    token: wintypes.HANDLE,
    user: str,
    *,
    diagnostics: list[str] | None = None,
) -> bool:
    """Load the run user's profile (NTUSER.DAT -> HKCU) for the logon token.

    S4 finding: a child spawned from a batch logon without profile load sees
    HKCU = the volatile .Default hive (writable), so the high-nist
    'hkcufrozen' assertion fails.  Loading the profile maps the child's HKCU
    to the run user's hive, which apply_hardening.ps1 freezes (deny-write
    ACE stored inside NTUSER.DAT).  Best-effort: failures are logged, not
    fatal (the file-level home freeze still applies).
    """
    if os.name != "nt" or not token:
        return False
    try:
        userenv = ctypes.WinDLL("userenv", use_last_error=True)

        class PROFILEINFO(ctypes.Structure):
            _fields_ = [
                ("dwSize", wintypes.DWORD),
                ("dwFlags", wintypes.DWORD),
                ("lpUserName", wintypes.LPWSTR),
                ("lpProfilePath", wintypes.LPWSTR),
                ("lpDefaultPath", wintypes.LPWSTR),
                ("lpServerName", wintypes.LPWSTR),
                ("lpPolicyPath", wintypes.LPWSTR),
                ("hProfile", wintypes.HANDLE),
            ]

        userenv.LoadUserProfileW.argtypes = [
            wintypes.HANDLE,
            ctypes.POINTER(PROFILEINFO),
        ]
        userenv.LoadUserProfileW.restype = wintypes.BOOL
        userenv.UnloadUserProfile.argtypes = [
            wintypes.HANDLE,
            wintypes.HANDLE,
        ]
        userenv.UnloadUserProfile.restype = wintypes.BOOL

        info = PROFILEINFO()
        info.dwSize = ctypes.sizeof(PROFILEINFO)
        info.lpUserName = wintypes.LPWSTR(user)
        if not userenv.LoadUserProfileW(token, ctypes.byref(info)):
            if diagnostics is not None:
                diagnostics.append(
                    f"LoadUserProfileW({user}) failed: "
                    f"{ctypes.WinError(ctypes.get_last_error())}"
                )
            return False
        return True
    except OSError as exc:
        if diagnostics is not None:
            diagnostics.append(f"LoadUserProfileW exception: {exc}")
        return False


def _enable_process_privilege(name: str) -> bool:
    """Enable a privilege (e.g. SeIncreaseQuotaPrivilege) in the current
    process token.  CreateProcessAsUserW with a different-user token requires
    SeIncreaseQuotaPrivilege (and SeAssignPrimaryTokenPrivilege when present).
    """
    if os.name != "nt":
        return False
    try:
        kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
        advapi32 = ctypes.WinDLL("advapi32", use_last_error=True)
        tok = wintypes.HANDLE()
        kernel32.OpenProcessToken.argtypes = [
            wintypes.HANDLE,
            wintypes.DWORD,
            ctypes.POINTER(wintypes.HANDLE),
        ]
        kernel32.OpenProcessToken.restype = wintypes.BOOL
        if not kernel32.OpenProcessToken(
            kernel32.GetCurrentProcess(),
            TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY,
            ctypes.byref(tok),
        ):
            return False
        try:
            luid = _LUID()
            advapi32.LookupPrivilegeValueW.argtypes = [
                wintypes.LPCWSTR,
                wintypes.LPCWSTR,
                ctypes.POINTER(_LUID),
            ]
            advapi32.LookupPrivilegeValueW.restype = wintypes.BOOL
            if not advapi32.LookupPrivilegeValueW(None, name, ctypes.byref(luid)):
                return False
            tp = _LUID_AND_ATTRIBUTES()
            tp.Luid = luid
            tp.Attributes = SE_PRIVILEGE_ENABLED
            advapi32.AdjustTokenPrivileges.argtypes = [
                wintypes.HANDLE,
                wintypes.BOOL,
                ctypes.c_void_p,
                wintypes.DWORD,
                ctypes.c_void_p,
                ctypes.c_void_p,
            ]
            advapi32.AdjustTokenPrivileges.restype = wintypes.BOOL
            ok = advapi32.AdjustTokenPrivileges(
                tok, False, ctypes.byref(tp), ctypes.sizeof(tp), None, None
            )
            # AdjustTokenPrivileges can return TRUE even if no privilege was
            # adjusted; check the last error for ERROR_SUCCESS (0).
            return bool(ok) and ctypes.get_last_error() == 0
        finally:
            kernel32.CloseHandle(tok)
    except OSError:
        return False


def _build_run_user_env(
    workspace: Path,
    arm: str,
    domain: str,
    user: str,
    extra: dict[str, str] | None,
) -> dict[str, str]:
    """Environment for a child spawned under the standard run user."""
    env = dict(os.environ)
    profile = rf"C:\Users\{user}"
    env["USERNAME"] = user
    env["USERDOMAIN"] = domain
    env["USERPROFILE"] = profile
    env["HOMEDRIVE"] = "C:"
    env["HOMEPATH"] = rf"\Users\{user}"
    env["APPDATA"] = rf"{profile}\AppData\Roaming"
    env["LOCALAPPDATA"] = rf"{profile}\AppData\Local"
    if arm == "high-nist":
        tmp = workspace / ".tmp"
        tmp.mkdir(parents=True, exist_ok=True)
        env["TEMP"] = str(tmp)
        env["TMP"] = str(tmp)
    else:
        env["TEMP"] = rf"{profile}\AppData\Local\Temp"
        env["TMP"] = rf"{profile}\AppData\Local\Temp"
    if extra:
        env.update(extra)
    return env


def _build_sandbox_env(
    workspace: Path, arm: str, extra: dict[str, str] | None
) -> dict[str, str]:
    env = dict(os.environ)
    if arm == "high-nist":
        tmp = workspace / ".tmp"
        tmp.mkdir(parents=True, exist_ok=True)
        env["TEMP"] = str(tmp)
        env["TMP"] = str(tmp)
    if extra:
        env.update(extra)
    return env


def _build_environment_block(env: dict[str, str]) -> bytes:
    items: list[str] = []
    for key, value in env.items():
        if not key or "=" in key or "\x00" in key:
            continue
        if "\x00" in str(value):
            continue
        items.append(f"{key}={value}")
    return ("\x00".join(items) + "\x00\x00").encode("utf-16-le")


def _command_line_from(command: list[str] | str) -> str:
    if isinstance(command, str):
        return command
    return subprocess.list2cmdline([str(item) for item in command])


def _create_anonymous_pipe() -> tuple[wintypes.HANDLE, wintypes.HANDLE] | None:
    if os.name != "nt":
        return None
    try:
        kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
        kernel32.CreatePipe.argtypes = [
            ctypes.POINTER(wintypes.HANDLE),
            ctypes.POINTER(wintypes.HANDLE),
            ctypes.c_void_p,
            wintypes.DWORD,
        ]
        kernel32.CreatePipe.restype = wintypes.BOOL
        read_h = wintypes.HANDLE()
        write_h = wintypes.HANDLE()
        if not kernel32.CreatePipe(ctypes.byref(read_h), ctypes.byref(write_h), None, 0):
            return None
        kernel32.SetHandleInformation.argtypes = [
            wintypes.HANDLE, wintypes.DWORD, wintypes.DWORD,
        ]
        kernel32.SetHandleInformation.restype = wintypes.BOOL
        kernel32.SetHandleInformation(write_h, HANDLE_FLAG_INHERIT, HANDLE_FLAG_INHERIT)
        kernel32.SetHandleInformation(read_h, HANDLE_FLAG_INHERIT, 0)
        return read_h, write_h
    except OSError as exc:
        return None


def _open_nul_read() -> wintypes.HANDLE | None:
    if os.name != "nt":
        return None
    try:
        kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
        kernel32.CreateFileW.argtypes = [
            wintypes.LPCWSTR,
            wintypes.DWORD,
            wintypes.DWORD,
            ctypes.c_void_p,
            wintypes.DWORD,
            wintypes.DWORD,
            wintypes.HANDLE,
        ]
        kernel32.CreateFileW.restype = wintypes.HANDLE
        handle = kernel32.CreateFileW(
            "NUL", 0x80000000, 0x3, None, 3, 0, None
        )
        return _as_handle(handle) if handle else None
    except OSError:
        return None


def _drain_pipe_handle(
    handle: wintypes.HANDLE, sink: list[bytes]
) -> None:
    if not handle:
        return
    try:
        import msvcrt

        fd = msvcrt.open_osfhandle(handle.value, os.O_RDONLY)
        with os.fdopen(fd, "rb", 65536) as stream:
            while True:
                chunk = stream.read(65536)
                if not chunk:
                    break
                sink.append(chunk)
    except OSError:
        pass


def _create_egress_allow_rules(
    allowlist_ips: list[str], *, diagnostics: list[str] | None = None
) -> bool:
    """Add per-IP outbound allow rules (host must use blockoutbound default).

    Windows Firewall semantics: block rules win over allow rules, so a blanket
    block-all rule would swallow the allowlist.  The hardening script therefore
    sets the host default outbound policy to block, and the run environment only
    adds allow rules for the per-task allowlist (design §3 Network row).
    """
    if os.name != "nt":
        return False
    if not _is_elevated():
        if diagnostics is not None:
            diagnostics.append(
                "egress allowlist rules require administrator elevation"
            )
        return False
    ok = True
    for ip in allowlist_ips:
        rule_name = f"{_FIREWALL_RULE_PREFIX}-Allow-{ip}"
        try:
            result = subprocess.run(
                [
                    "netsh", "advfirewall", "firewall", "add", "rule",
                    f"name={rule_name}",
                    "dir=out",
                    "action=allow",
                    f"remoteip={ip}",
                    "profile=any",
                    "enable=yes",
                ],
                capture_output=True,
                shell=False,
                timeout=15,
            )
            if result.returncode != 0:
                ok = False
                out = (result.stdout or b"").decode(
                    "utf-8", errors="replace"
                ).strip()
                err = (result.stderr or b"").decode(
                    "utf-8", errors="replace"
                ).strip()
                if diagnostics is not None:
                    diagnostics.append(
                        f"allow rule {ip} failed (rc={result.returncode}): "
                        f"{out[:160]} {err[:160]}"
                    )
        except Exception as exc:
            ok = False
            if diagnostics is not None:
                diagnostics.append(f"allow rule {ip} failed: {exc}")
    return ok


def _delete_egress_allow_rules(
    allowlist_ips: list[str], *, diagnostics: list[str] | None = None
) -> None:
    if os.name != "nt":
        return
    for ip in allowlist_ips:
        _delete_firewall_rule(
            f"{_FIREWALL_RULE_PREFIX}-Allow-{ip}", diagnostics=diagnostics
        )


def run_windows_native_sandbox(
    command: list[str] | str,
    workspace: Path,
    *,
    arm: str = "high-nist",
    profile_path: Path | None = None,
    cwd: Path | None = None,
    env: dict[str, str] | None = None,
    timeout_seconds: int | None = None,
    memory_limit_bytes: int | None = None,
    allowlist_ips: list[str] | None = None,
    capture_output: bool = True,
) -> dict[str, Any]:
    """Spawn a command tree in the Windows hardened run environment.

    arms:
      control    — current token, no policy wall (still Job-contained).
      non-admin  — restricted token (Administrators disabled/deny-only,
                   six privileges removed, TokenVirtualizationAllowed=0).
      high-nist  — non-admin + LOW integrity + AppContainer (empty
                   capabilities) + Job Object + %TEMP% redirect + egress wall.

    Returns a run observation (see windows-native-sandbox-run-v0.1.schema.json).
    """
    if os.name != "nt":
        raise AssuranceError(
            "Windows native sandbox run environment requires Windows"
        )
    if arm not in WINDOWS_RUN_ARMS:
        raise AssuranceError(
            f"unknown arm {arm!r} (expected control|non-admin|high-nist)"
        )

    profile = _load_windows_native_profile(profile_path)
    resolved_workspace = _validate_disposable_workspace(workspace)
    wall_timeout = (
        int(timeout_seconds)
        if timeout_seconds is not None
        else int(profile["resources"]["wall_time_seconds"])
    )
    if wall_timeout <= 0:
        wall_timeout = 1
    memory_limit = (
        int(memory_limit_bytes)
        if memory_limit_bytes is not None
        else int(profile["resources"]["memory_bytes"])
    )

    observation_id = f"WNR-{uuid.uuid4().hex.upper()}"
    probe_diags: list[str] = []
    command_line = _command_line_from(command)
    run_user = _get_run_user_credentials()
    cmd_env = _build_sandbox_env(resolved_workspace, arm, env)
    if run_user and arm != "control":
        cmd_env = _build_run_user_env(
            resolved_workspace, arm, run_user[0], run_user[1], env
        )
    run_cwd = (cwd or resolved_workspace).resolve()
    env_block: bytes | None = None
    if arm == "high-nist" or (run_user and arm != "control"):
        env_block = _build_environment_block(cmd_env)

    app_name = f"p2_native_run_{uuid.uuid4().hex[:16]}"
    appcontainer_sid: ctypes.c_void_p | None = None
    profile_created = False
    profile_deleted = False
    sid_derived = False
    job: wintypes.HANDLE | None = None
    job_created = False
    job_assigned = False
    creation_time_job_assigned = False
    firewall_rule_name: str | None = None
    firewall_rule_created = False
    firewall_diagnostic = "not attempted"
    restricted_token: wintypes.HANDLE | None = None
    primary_token: wintypes.HANDLE | None = None
    run_user_logon_token: wintypes.HANDLE | None = None
    process_handle: wintypes.HANDLE | None = None
    thread_handle: wintypes.HANDLE | None = None
    process_pid = 0
    exit_code: int | None = None
    timed_out = False
    cancellation_method: str | None = None
    ctrl_break_sent = False
    ctrl_break_effective = False
    read_pipe: wintypes.HANDLE | None = None
    write_pipe: wintypes.HANDLE | None = None
    err_read_pipe: wintypes.HANDLE | None = None
    err_write_pipe: wintypes.HANDLE | None = None
    nul_handle: wintypes.HANDLE | None = None
    stdout_chunks: list[bytes] = []
    stderr_chunks: list[bytes] = []
    readers: list[threading.Thread] = []
    checks: dict[str, bool] = {}
    created_with_restricted_token = False
    allowed_ips = list(allowlist_ips or [])

    def _diag(message: str) -> None:
        probe_diags.append(message)

    try:
        # Harness-side workspace writable fact（grant 后由宿主写探针文件）。
        ws_probe = resolved_workspace / f".run-ws-probe-{uuid.uuid4().hex[:8]}"
        try:
            ws_probe.write_text("ok", encoding="utf-8")
            ws_probe.unlink()
            checks["workspace_writable"] = True
        except OSError as exc:
            checks["workspace_writable"] = False
            _diag(f"workspace probe write failed: {exc}")

        if arm != "control":
            if run_user:
                grant_ok = _grant_workspace_access_account(
                    resolved_workspace,
                    run_user[0],
                    run_user[1],
                    diagnostics=probe_diags,
                )
            else:
                grant_ok = _grant_workspace_access_current_user(
                    resolved_workspace, diagnostics=probe_diags
                )
            if not grant_ok:
                _diag("workspace user ACL grant failed (run user may lack access)")

        if arm == "high-nist":
            appcontainer_sid = _derive_appcontainer_sid(
                app_name, diagnostics=probe_diags
            )
            if appcontainer_sid:
                sid_derived = True
            created_sid = _create_appcontainer_profile(
                app_name, diagnostics=probe_diags
            )
            if created_sid:
                if appcontainer_sid:
                    _free_sid(appcontainer_sid, diagnostics=probe_diags)
                appcontainer_sid = created_sid
                profile_created = True
            if not sid_derived and not profile_created:
                raise AssuranceError(
                    "Windows native run environment cannot derive or create "
                    "AppContainer SID (high-nist arm)"
                )
            grant_ok = _grant_appcontainer_workspace_access(
                resolved_workspace, appcontainer_sid, diagnostics=probe_diags
            )
            if not grant_ok:
                _diag("AppContainer workspace ACL grant failed (fail-closed)")
            # S4 finding: LOW-integrity children cannot write to default
            # (Medium) labeled objects (mandatory-integrity no-write-up), so
            # the AppContainer child could read but not write the workspace.
            # Label the workspace tree Low so probe output lands there.
            try:
                subprocess.run(
                    [
                        "icacls",
                        str(resolved_workspace),
                        "/setintegritylevel",
                        "Low",
                        "/T",
                        "/C",
                    ],
                    capture_output=True,
                    shell=False,
                    timeout=30,
                )
            except Exception as exc:
                _diag(f"workspace low-integrity label failed: {exc}")
            if run_user:
                # S4 finding: CreateAppContainerProfile creates the package
                # under the CALLING user's profile (SYSTEM here), but the
                # OS rewrites TEMP/LOCALAPPDATA for the AppContainer child to
                # <RunUser>\AppData\Local\Packages\<app>\AC — which does not
                # exist and is under the frozen home.  Pre-create the package
                # AC\Temp in the run user's profile and grant the package SID.
                try:
                    pkg_root = (
                        Path(os.environ.get("SystemDrive", "C:") + "\\")
                        / "Users"
                        / run_user[1]
                        / "AppData"
                        / "Local"
                        / "Packages"
                        / app_name
                    )
                    ac_temp = pkg_root / "AC" / "Temp"
                    ac_temp.mkdir(parents=True, exist_ok=True)
                    appc_sid_str = _appcontainer_sid_to_string(appcontainer_sid)
                    if appc_sid_str:
                        subprocess.run(
                            [
                                "icacls",
                                str(pkg_root),
                                "/grant",
                                f"*{appc_sid_str}:(OI)(CI)(M)",
                                "/T",
                                "/C",
                            ],
                            capture_output=True,
                            shell=False,
                            timeout=20,
                        )
                except Exception as exc:
                    _diag(f"AppContainer package temp setup failed: {exc}")

        if arm != "control":
            if run_user:
                run_user_logon_token = _logon_run_user(
                    run_user, diagnostics=probe_diags
                )
                if run_user_logon_token:
                    # CreateProcessWithTokenW consumes the LogonUser token
                    # directly (impersonation level) and only needs
                    # SeImpersonatePrivilege, which admin tokens hold.
                    if not _set_token_virtualization(
                        run_user_logon_token, False, diagnostics=probe_diags
                    ):
                        _diag(
                            "TokenVirtualizationAllowed=0 could not be set "
                            "(fail-closed)"
                        )
                    if arm == "high-nist":
                        if not _set_token_integrity(
                            run_user_logon_token,
                            SECURITY_MANDATORY_LOW_RID,
                            diagnostics=probe_diags,
                        ):
                            _diag(
                                "LOW integrity level could not be set (fail-closed)"
                            )
            if run_user_logon_token:
                # Load the run user's profile so the child's HKCU maps to the
                # frozen hive (hkcufrozen assertion) instead of the volatile
                # .Default hive.  Best-effort (logged, not fatal).
                _load_run_user_profile(
                    run_user_logon_token, run_user[1], diagnostics=probe_diags
                )
                # Session-0 scheduled-task finding: the service window
                # station/desktop ACL grants only the task owner +
                # Administrators, so a child spawned with the run-user token
                # fails DLL init (0xC0000142).  Grant the run user access to
                # the current winsta/desktop before spawning (per-session).
                account = (
                    f"{run_user[0]}\\{run_user[1]}"
                    if run_user[0]
                    else run_user[1]
                )
                desktop_extra: list[str] = []
                if arm == "high-nist" and appcontainer_sid:
                    appc_sid_str = _appcontainer_sid_to_string(appcontainer_sid)
                    if appc_sid_str:
                        desktop_extra.append(appc_sid_str)
                if not _grant_session_desktop_access(
                    account,
                    extra_sids=desktop_extra,
                    diagnostics=probe_diags,
                ):
                    _diag(
                        "run-user window-station/desktop grant failed "
                        "(spawn will likely fail with 0xC0000142)"
                    )
            if not primary_token and not run_user_logon_token:
                spec = build_restricted_token_spec(arm)
                restricted_token = _create_restricted_token(
                    spec, diagnostics=probe_diags
                )
                if restricted_token:
                    primary_token = _duplicate_token_primary(
                        restricted_token, diagnostics=probe_diags
                    )
                if arm == "high-nist" and primary_token:
                    if not _set_token_integrity(
                        primary_token,
                        SECURITY_MANDATORY_LOW_RID,
                        diagnostics=probe_diags,
                    ):
                        _diag("LOW integrity level could not be set (fail-closed)")
            if not primary_token and not run_user_logon_token:
                _diag(
                    "run-user/restricted token unavailable — degrading to "
                    "current-token "
                    "spawn (run observation will be noncompliant)"
                )

        job = _create_kill_on_close_job(
            memory_limit,
            active_process_limit=profile["resources"]["pids_limit"],
        )
        if job:
            job_created = True

        if arm == "high-nist":
            if allowed_ips:
                firewall_rule_created = _create_egress_allow_rules(
                    allowed_ips, diagnostics=probe_diags
                )
                firewall_diagnostic = (
                    "allowlist-rule mode; host firewall policy must be "
                    "blockoutbound (apply_hardening.ps1 sets it)"
                )
            else:
                sid_string = _appcontainer_sid_to_string(appcontainer_sid)
                if sid_string:
                    firewall_rule_name, firewall_rule_created, firewall_diagnostic = (
                        _create_firewall_outbound_block_rule(app_name, sid_string)
                    )

        # S4 finding: AppContainer children fail DLL initialization
        # (STATUS_DLL_INIT_FAILED / 0xC0000142) when std handles are the
        # inherited anonymous pipes, because the pipe DACL grants only the
        # creating user (SYSTEM) and the AppContainer token cannot access
        # the handles.  The probe writes its result to the workspace, so
        # stdout/stderr capture is skipped for AppContainer spawns.
        use_std_handles = capture_output and not (
            arm == "high-nist" and run_user_logon_token
        )
        if use_std_handles:
            out_pair = _create_anonymous_pipe()
            if out_pair:
                read_pipe, write_pipe = out_pair
            err_pair = _create_anonymous_pipe()
            if err_pair:
                err_read_pipe, err_write_pipe = err_pair
            nul_handle = _open_nul_read()

        kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)

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

        if arm == "high-nist":
            sec_cap = SECURITY_CAPABILITIES()
            sec_cap.AppContainerSid = appcontainer_sid
            sec_cap.Capabilities = None
            sec_cap.CapabilityCount = 0
            sec_cap.Reserved = 0
            if not kernel32.UpdateProcThreadAttribute(
                ctypes.cast(attr_list, ctypes.c_void_p),
                0,
                ctypes.c_size_t(PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES),
                ctypes.byref(sec_cap),
                ctypes.sizeof(sec_cap),
                None,
                None,
            ):
                raise AssuranceError(
                    "cannot set PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES"
                )

        if job:
            if not kernel32.UpdateProcThreadAttribute(
                ctypes.cast(attr_list, ctypes.c_void_p),
                0,
                ctypes.c_size_t(PROC_THREAD_ATTRIBUTE_JOB_LIST),
                ctypes.byref(job),
                ctypes.sizeof(wintypes.HANDLE),
                None,
                None,
            ):
                _diag(
                    "PROC_THREAD_ATTRIBUTE_JOB_LIST not supported; "
                    "falling back to post-creation AssignProcessToJobObject"
                )
            else:
                creation_time_job_assigned = True

        si_ex = STARTUPINFOEX()
        si_ex.StartupInfo.cb = ctypes.sizeof(STARTUPINFOEX)
        si_ex.lpAttributeList = ctypes.cast(attr_list, ctypes.c_void_p)
        if use_std_handles and write_pipe:
            si_ex.StartupInfo.dwFlags |= STARTF_USESTDHANDLES
            si_ex.StartupInfo.hStdOutput = write_pipe
            si_ex.StartupInfo.hStdError = err_write_pipe or write_pipe
            si_ex.StartupInfo.hStdInput = nul_handle or wintypes.HANDLE(0)

        proc_info = PROCESS_INFORMATION()
        cmd_buf = ctypes.create_unicode_buffer(command_line)
        creation_flags = (
            EXTENDED_STARTUPINFO_PRESENT_FLAG
            | CREATE_SUSPENDED_FLAG
            | CREATE_NO_WINDOW_FLAG
            | CREATE_NEW_PROCESS_GROUP_FLAG
        )
        if env_block is not None:
            # The environment block is UTF-16; without this flag Windows
            # parses it as ANSI and rejects it (ERROR_INVALID_PARAMETER /
            # ERROR_ENVVAR_NOT_FOUND observed in S4).
            creation_flags |= CREATE_UNICODE_ENVIRONMENT_FLAG

        spawn_ok = False
        env_ptr = None
        if run_user_logon_token:
            # Preferred: CreateProcessAsUserW with the LogonUser primary
            # token.  Supports the extended startup info required for
            # AppContainer and creation-time Job containment.  Requires the
            # caller to hold SeAssignPrimaryTokenPrivilege (present in SYSTEM
            # scheduled-task tokens, absent from admin-user task tokens) —
            # S4 finding: run the sandbox job as SYSTEM.
            advapi32 = ctypes.WinDLL("advapi32", use_last_error=True)
            advapi32.CreateProcessAsUserW.argtypes = [
                wintypes.HANDLE,
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
            advapi32.CreateProcessAsUserW.restype = wintypes.BOOL
            env_ptr = (
                ctypes.cast(ctypes.c_char_p(env_block), ctypes.c_void_p)
                if env_block is not None
                else None
            )
            _enable_process_privilege("SeIncreaseQuotaPrivilege")
            spawn_ok = advapi32.CreateProcessAsUserW(
                run_user_logon_token,
                None,
                cmd_buf,
                None,
                None,
                True,
                creation_flags,
                env_ptr,
                str(run_cwd),
                ctypes.cast(ctypes.byref(si_ex), ctypes.c_void_p),
                ctypes.byref(proc_info),
            )
            if not spawn_ok:
                _diag(
                    "CreateProcessAsUserW failed: "
                    f"{ctypes.WinError(ctypes.get_last_error())}; "
                    "falling back to CreateProcessWithTokenW (plain startup "
                    "info — extended attributes are rejected by that API)"
                )
                # CreateProcessWithTokenW rejects STARTUPINFOEX /
                # EXTENDED_STARTUPINFO_PRESENT with ERROR_INVALID_PARAMETER
                # on this build, so rebuild a plain STARTUPINFO (Job
                # containment then relies on the post-creation
                # AssignProcessToJobObject fallback below; AppContainer
                # cannot be applied on this fallback path).
                plain_flags = creation_flags & ~EXTENDED_STARTUPINFO_PRESENT_FLAG
                si_plain = STARTUPINFOW()
                si_plain.cb = ctypes.sizeof(STARTUPINFOW)
                if use_std_handles and write_pipe:
                    si_plain.dwFlags |= STARTF_USESTDHANDLES
                    si_plain.hStdOutput = write_pipe
                    si_plain.hStdError = err_write_pipe or write_pipe
                    si_plain.hStdInput = nul_handle or wintypes.HANDLE(0)
                advapi32.CreateProcessWithTokenW.argtypes = [
                    wintypes.HANDLE,
                    wintypes.DWORD,
                    wintypes.LPCWSTR,
                    wintypes.LPWSTR,
                    wintypes.DWORD,
                    ctypes.c_void_p,
                    wintypes.LPCWSTR,
                    ctypes.c_void_p,
                    ctypes.POINTER(PROCESS_INFORMATION),
                ]
                advapi32.CreateProcessWithTokenW.restype = wintypes.BOOL
                spawn_ok = advapi32.CreateProcessWithTokenW(
                    run_user_logon_token,
                    0,
                    None,
                    cmd_buf,
                    plain_flags,
                    env_ptr,
                    str(run_cwd),
                    ctypes.cast(ctypes.byref(si_plain), ctypes.c_void_p),
                    ctypes.byref(proc_info),
                )
                if not spawn_ok:
                    _diag(
                        "CreateProcessWithTokenW fallback failed: "
                        f"{ctypes.WinError(ctypes.get_last_error())}"
                    )
        if run_user_logon_token and spawn_ok:
            # Run-user logon spawns satisfy the restricted-token contract
            # (standard user, no admin group, virtualization off).
            created_with_restricted_token = True
        elif primary_token:
            # CreateProcessAsUserW: primary token + extended startup info
            # (AppContainer attr for high-nist, Job list for all arms).
            advapi32 = ctypes.WinDLL("advapi32", use_last_error=True)
            advapi32.CreateProcessAsUserW.argtypes = [
                wintypes.HANDLE,
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
            advapi32.CreateProcessAsUserW.restype = wintypes.BOOL
            env_ptr = (
                ctypes.cast(ctypes.c_char_p(env_block), ctypes.c_void_p)
                if env_block is not None
                else None
            )
            spawn_ok = advapi32.CreateProcessAsUserW(
                primary_token,
                None,
                cmd_buf,
                None,
                None,
                True,
                creation_flags,
                env_ptr,
                str(run_cwd),
                ctypes.cast(ctypes.byref(si_ex), ctypes.c_void_p),
                ctypes.byref(proc_info),
            )
            if spawn_ok:
                created_with_restricted_token = True
            else:
                _diag(
                    "CreateProcessAsUserW failed: "
                    f"{ctypes.WinError(ctypes.get_last_error())}"
                )

        if not spawn_ok:
            # Degraded fallback: current token + AppContainer attribute.
            # Fail-closed: spawn-time checks will be false for restricted
            # token facts, so the run observation is noncompliant.
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
            spawn_ok = kernel32.CreateProcessW(
                None,
                cmd_buf,
                None,
                None,
                True,
                creation_flags,
                env_ptr if env_block is not None else None,
                str(run_cwd),
                ctypes.cast(ctypes.byref(si_ex), ctypes.c_void_p),
                ctypes.byref(proc_info),
            )
            if not spawn_ok:
                raise AssuranceError(
                    "Windows native run environment process creation failed: "
                    f"{ctypes.WinError(ctypes.get_last_error())}"
                )

        kernel32.DeleteProcThreadAttributeList.argtypes = [ctypes.c_void_p]
        kernel32.DeleteProcThreadAttributeList.restype = None
        kernel32.DeleteProcThreadAttributeList(ctypes.cast(attr_list, ctypes.c_void_p))

        process_handle = proc_info.hProcess
        thread_handle = proc_info.hThread
        process_pid = proc_info.dwProcessId

        # Parent must close its write copy so readers see EOF when child exits.
        if write_pipe:
            _close_handle(write_pipe)
            write_pipe = None
        if err_write_pipe:
            _close_handle(err_write_pipe)
            err_write_pipe = None

        if capture_output:
            if read_pipe:
                t_out = threading.Thread(
                    target=_drain_pipe_handle, args=(read_pipe, stdout_chunks)
                )
                t_out.daemon = True
                t_out.start()
                readers.append(t_out)
            if err_read_pipe:
                t_err = threading.Thread(
                    target=_drain_pipe_handle, args=(err_read_pipe, stderr_chunks)
                )
                t_err.daemon = True
                t_err.start()
                readers.append(t_err)

        # Spawn-time verification before ResumeThread（fail-closed）。
        proc_token = wintypes.HANDLE()
        kernel32.OpenProcessToken.argtypes = [
            wintypes.HANDLE,
            wintypes.DWORD,
            ctypes.POINTER(wintypes.HANDLE),
        ]
        kernel32.OpenProcessToken.restype = wintypes.BOOL
        if kernel32.OpenProcessToken(
            process_handle, TOKEN_QUERY, ctypes.byref(proc_token)
        ):
            try:
                if arm != "control":
                    checks["non_admin"] = not bool(
                        _token_has_enabled_group(
                            proc_token, "S-1-5-32-544", diagnostics=probe_diags
                        )
                    )
                    virt = _token_bool(
                        proc_token, TokenVirtualizationAllowed, diagnostics=probe_diags
                    )
                    checks["token_virtualization_disabled"] = virt is False
                    enabled = _token_enabled_privileges(
                        proc_token, diagnostics=probe_diags
                    )
                    checks["privileges_removed"] = not any(
                        name in WINDOWS_RUN_RESTRICTED_PRIVILEGES for name in enabled
                    )
                if arm == "high-nist":
                    rid = _token_integrity_rid(proc_token, diagnostics=probe_diags)
                    checks["low_integrity"] = (
                        rid is not None and rid <= SECURITY_MANDATORY_LOW_RID
                    )
                    checks["appcontainer_token"] = _process_token_is_appcontainer(
                        process_handle, diagnostics=probe_diags
                    )
                    tmp_path = cmd_env.get("TEMP", "")
                    checks["temp_redirected"] = bool(
                        tmp_path
                        and str(resolved_workspace / ".tmp").lower()
                        == str(Path(tmp_path)).lower()
                        and (resolved_workspace / ".tmp").is_dir()
                    )
            finally:
                kernel32.CloseHandle(proc_token)
        else:
            _diag("could not open child token for spawn-time checks (fail-closed)")

        if arm == "high-nist":
            checks["job_object_assigned"] = _is_process_in_job(
                process_handle, diagnostics=probe_diags
            )
        elif arm != "control":
            # Job containment is required for all arms (process-tree cleanup).
            checks["job_object_assigned"] = _is_process_in_job(
                process_handle, diagnostics=probe_diags
            )

        if job:
            if creation_time_job_assigned:
                already_in = _is_process_in_job(
                    process_handle, diagnostics=probe_diags
                )
                if already_in:
                    job_assigned = True
                else:
                    _diag(
                        "creation-time Job assignment did not take effect; "
                        "attempting post-creation assignment"
                    )
                    job_assigned = _assign_process_to_job(job, process_handle)
            else:
                job_assigned = _assign_process_to_job(job, process_handle)
            if not job_assigned:
                _terminate_suspended_process(process_handle, thread_handle)
                process_handle = None
                thread_handle = None
                raise AssuranceError(
                    "Windows native run environment failed to assign process "
                    "to Job Object"
                )

        kernel32.ResumeThread.argtypes = [wintypes.HANDLE]
        kernel32.ResumeThread.restype = wintypes.DWORD
        if kernel32.ResumeThread(thread_handle) == 0xFFFFFFFF:
            err = ctypes.WinError(ctypes.get_last_error())
            _terminate_suspended_process(process_handle, thread_handle)
            process_handle = None
            thread_handle = None
            raise AssuranceError(f"Windows native run environment ResumeThread failed: {err}")
        kernel32.CloseHandle(thread_handle)
        thread_handle = None

        kernel32.WaitForSingleObject.argtypes = [wintypes.HANDLE, wintypes.DWORD]
        kernel32.WaitForSingleObject.restype = wintypes.DWORD
        wait_result = kernel32.WaitForSingleObject(
            process_handle, wintypes.DWORD(wall_timeout * 1000)
        )

        if wait_result == 0x00000102:
            timed_out = True
            cancellation_method = "timeout"
            kernel32.GenerateConsoleCtrlEvent.argtypes = [
                wintypes.DWORD,
                wintypes.DWORD,
            ]
            kernel32.GenerateConsoleCtrlEvent.restype = wintypes.BOOL
            ctrl_break_sent = kernel32.GenerateConsoleCtrlEvent(
                wintypes.DWORD(CTRL_BREAK_EVENT),
                wintypes.DWORD(process_pid),
            )
            _diag(f"CTRL_BREAK_EVENT sent={ctrl_break_sent} to pid={process_pid}")
            if ctrl_break_sent:
                grace_result = kernel32.WaitForSingleObject(
                    process_handle, wintypes.DWORD(CANCEL_GRACE_SECONDS * 1000)
                )
                ctrl_break_effective = grace_result == 0
                if ctrl_break_effective:
                    cancellation_method = "ctrl_break"
            if not ctrl_break_effective:
                if job and job_assigned:
                    cancellation_method = "job_close"
                    _close_handle(job)
                    job = None
                else:
                    cancellation_method = "terminate_process"
                    kernel32.TerminateProcess.argtypes = [
                        wintypes.HANDLE,
                        wintypes.UINT,
                    ]
                    kernel32.TerminateProcess.restype = wintypes.BOOL
                    kernel32.TerminateProcess(process_handle, 1)
            kernel32.WaitForSingleObject(process_handle, wintypes.DWORD(5000))

        kernel32.GetExitCodeProcess.argtypes = [wintypes.HANDLE, ctypes.POINTER(wintypes.DWORD)]
        kernel32.GetExitCodeProcess.restype = wintypes.BOOL
        ec = wintypes.DWORD()
        if kernel32.GetExitCodeProcess(process_handle, ctypes.byref(ec)):
            exit_code = ec.value

        _close_handle(process_handle)
        process_handle = None

        for reader in readers:
            reader.join(timeout=5)
        if read_pipe:
            _close_handle(read_pipe)
            read_pipe = None
        if err_read_pipe:
            _close_handle(err_read_pipe)
            err_read_pipe = None

    finally:
        if firewall_rule_name:
            _delete_firewall_rule(firewall_rule_name, diagnostics=probe_diags)
        if allowed_ips:
            _delete_egress_allow_rules(allowed_ips, diagnostics=probe_diags)
        if job:
            _close_handle(job)
        if process_handle:
            _close_handle(process_handle)
        if thread_handle:
            _close_handle(thread_handle)
        if write_pipe:
            _close_handle(write_pipe)
        if read_pipe:
            _close_handle(read_pipe)
        if err_write_pipe:
            _close_handle(err_write_pipe)
        if err_read_pipe:
            _close_handle(err_read_pipe)
        if nul_handle:
            _close_handle(nul_handle)
        if restricted_token:
            _close_handle(restricted_token)
        if primary_token:
            _close_handle(primary_token)
        if run_user_logon_token:
            _close_handle(run_user_logon_token)
        if appcontainer_sid and not profile_created:
            _free_sid(appcontainer_sid, diagnostics=probe_diags)
        if profile_created:
            profile_deleted = _delete_appcontainer_profile(
                app_name, diagnostics=probe_diags
            )

    stdout_bytes = b"".join(stdout_chunks)
    stderr_bytes = b"".join(stderr_chunks)
    required_checks = set(run_observation_checks_for_arm(arm))
    observed_keys = set(checks.keys())
    all_required_present = observed_keys >= required_checks
    all_required_passed = all_required_present and all(
        checks.get(name) is True for name in required_checks
    )
    outcome = "compliant" if all_required_passed else "noncompliant"

    observation: dict[str, Any] = {
        "schema_version": "0.1.0-draft",
        "observation_kind": "windows_native_sandbox_run_observation",
        "observation_id": observation_id,
        "created_at": utc_now(),
        "arm": arm,
        "profile_sha256": sha256_file(
            profile_path or ASSURANCE_ROOT / "windows-native-sandbox-profile-v0.1.json"
        ),
        "workspace_path_sha256": sha256_bytes(
            str(resolved_workspace).encode("utf-8")
        ),
        "command_sha256": sha256_bytes(command_line.encode("utf-16-le")),
        "command": command_line,
        "cwd": str(run_cwd),
        "env_overrides": dict(env or {}),
        "temp_redirect": {
            "workspace_tmp": str(resolved_workspace / ".tmp"),
            "temp_env": cmd_env.get("TEMP", ""),
            "tmp_env": cmd_env.get("TMP", ""),
        },
        "token": {
            "restricted": created_with_restricted_token,
            "virtualization_allowed": not bool(
                checks.get("token_virtualization_disabled", False)
            ),
            "low_integrity": bool(checks.get("low_integrity", False)),
            "appcontainer": bool(checks.get("appcontainer_token", False)),
            "privileges_removed": bool(checks.get("privileges_removed", False)),
        },
        "appcontainer": {
            "sid_derived": sid_derived,
            "profile_created": profile_created,
            "profile_deleted": profile_deleted,
            "capabilities": [],
        },
        "job_object": {
            "created": job_created,
            "assigned": job_assigned,
            "creation_time_assignment": creation_time_job_assigned,
            "kill_on_close": job_created,
            "memory_limit_bytes": memory_limit,
            "active_process_limit": profile["resources"]["pids_limit"],
        },
        "firewall": {
            "outbound_block_rule_created": firewall_rule_created,
            "rule_name": firewall_rule_name or "",
            "allowlist_ips": allowed_ips,
            "diagnostic": firewall_diagnostic,
        },
        "process": {
            "pid": process_pid,
            "exit_code": exit_code if exit_code is not None else -1,
            "timed_out": timed_out,
            "cancellation_method": cancellation_method,
            "shell_used": False,
        },
        "output": {
            "stdout_bytes": len(stdout_bytes),
            "stderr_bytes": len(stderr_bytes),
            "stdout_sha256": sha256_bytes(stdout_bytes),
            "stderr_sha256": sha256_bytes(stderr_bytes),
        },
        "checks": {
            name: bool(checks.get(name, False)) for name in sorted(required_checks)
        },
        "outcome": outcome,
        "evidence_status": "observed",
        "diagnostics": probe_diags,
        "limitations": [
            "Run observation covers spawn-time facts only (token/IL/AppContainer/"
            "Job/TEMP redirect).  Behavioral wall assertions (System32/Program "
            "Files/HKLM/home/network) are performed by enforcement_probe.ps1 "
            "inside this environment.",
            "CreateProcessAsUserW requires administrator elevation for "
            "restricted-token spawns; degraded current-token spawns are "
            "fail-closed (outcome=noncompliant).",
            "AppContainer alone does not block raw TCP outbound on some Windows "
            "builds; network isolation relies on the host firewall wall "
            "(hardening script blockoutbound default + per-task allowlist).",
        ],
    }

    validate_contract(
        observation,
        "windows-native-sandbox-run-v0.1.schema.json",
        label="Windows native sandbox run observation",
    )

    from .sandbox_verifier import verify_windows_native_run_observation

    verification = verify_windows_native_run_observation(
        observation,
        profile=profile,
        profile_path=profile_path,
        require_compliant=(outcome == "compliant"),
    )
    if not verification["valid"]:
        raise AssuranceError(
            "Windows native run observation failed independent verification: "
            + "; ".join(verification["errors"])
        )
    return observation
