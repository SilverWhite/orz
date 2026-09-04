"""Windows enforcement-probe (Python port).

Equivalent of enforcement_probe.ps1, used as the sandboxed child for the
non-admin / high-nist arms.  The sandbox child must be AppContainer-capable:
powershell.exe (Windows PowerShell 5.1 / .NET Framework CLR) fails DLL
initialization (STATUS_DLL_INIT_FAILED / 0xC0000142) as an AppContainer
process on Windows 11 25H2, while native programs (cmd, python) work.

Exit code: 0 = all assertions for the arm passed; 1 = any failed
(fail-closed).  Writes the same result JSON shape as the PS probe.

Usage:
  python enforcement_probe.py -Arm high-nist -Workspace C:\\workspace\\high-nist
      -TempDir C:\\workspace\\high-nist\\.tmp -ResultPath probe.json
      [-AllowlistReachabilityIp 221.204.163.76]
"""

import argparse
import json
import os
import random
import re
import socket
import subprocess as sp
import sys
import time
import traceback
import uuid
import winreg

RESTRICTED_PRIVILEGES = {
    "SeDebugPrivilege",
    "SeBackupPrivilege",
    "SeRestorePrivilege",
    "SeTakeOwnershipPrivilege",
    "SeLoadDriverPrivilege",
    "SeCreateSymbolicLinkPrivilege",
}


def _now_iso():
    from datetime import datetime, timezone

    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def _whoami(args, workspace=None):
    """Run whoami, redirecting output to a workspace file (no pipes).

    AppContainer children cannot use anonymous-pipe std handles (DLL init
    failure), so subprocess capture_output is unusable here.
    """
    out_file = os.path.join(
        workspace or os.getcwd(), f".whoami-{''.join(args).replace('/', '_')}.out"
    )
    try:
        with open(out_file, "w", encoding="utf-8") as fh:
            sp.run(["whoami.exe"] + args, stdout=fh, stderr=sp.STDOUT, timeout=15)
        with open(out_file, "r", encoding="utf-8", errors="replace") as fh:
            return fh.read()
    except OSError:
        return ""


def _is_admin(workspace=None) -> bool:
    return "S-1-5-32-544" in _whoami(["/groups"], workspace)


def _enabled_privileges(workspace=None) -> list[str]:
    out = _whoami(["/priv"], workspace)
    enabled: list[str] = []
    for line in out.splitlines():
        match = re.match(r"\s*(Se\w+Privilege)\s+", line)
        if match and ("已启用" in line or "Enabled" in line):
            enabled.append(match.group(1))
    return enabled


def _integrity_rid(workspace=None) -> int | None:
    out = _whoami(["/groups"], workspace)
    for line in out.splitlines():
        match = re.search(r"S-1-16-(\d+)", line)
        if match:
            return int(match.group(1))
    return None


def _is_appcontainer(workspace=None) -> bool:
    return bool(re.search(r"S-1-15-2-\d+", _whoami(["/groups"], workspace)))


def path_write_succeeded(path: str) -> bool:
    target = os.path.join(path, f".p2-probe-ok-{uuid.uuid4().hex[:8]}.tmp")
    try:
        with open(target, "w", encoding="utf-8") as fh:
            fh.write("x")
        os.remove(target)
        return True
    except OSError:
        return False


def path_write_blocked(path: str) -> bool:
    return not path_write_succeeded(path)


def registry_write_blocked(root, sub_path: str) -> bool:
    probe = os.path.join(sub_path, "_p2_ps_probe_del")
    try:
        key = winreg.CreateKeyEx(root, probe, 0, winreg.KEY_WRITE)
        winreg.CloseKey(key)
        try:
            winreg.DeleteKey(root, probe)
        except OSError:
            pass
        return False
    except OSError:
        return True


def tcp_blocked(ip: str, port: int, timeout_ms: int = 1500) -> bool:
    try:
        with socket.create_connection((ip, port), timeout_ms / 1000.0) as sock:
            return not bool(sock)
    except OSError:
        return True


def uac_hardened() -> bool:
    try:
        with winreg.OpenKey(
            winreg.HKEY_LOCAL_MACHINE,
            r"SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\System",
        ) as key:
            enable_lua = int(winreg.QueryValueEx(key, "EnableLUA")[0])
            consent = int(winreg.QueryValueEx(key, "ConsentPromptBehaviorAdmin")[0])
            return enable_lua == 1 and consent == 2
    except OSError:
        return False


def virtual_store_has_probe(local_appdata: str) -> bool:
    store = os.path.join(local_appdata, "VirtualStore")
    if not os.path.isdir(store):
        return False
    for root, _dirs, files in os.walk(store):
        for name in files:
            if name.startswith(".p2-probe-w-"):
                return True
    return False


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("-Arm", required=True)
    parser.add_argument("-Workspace", default="")
    parser.add_argument("-TempDir", default="")
    parser.add_argument("-ResultPath", default="")
    parser.add_argument("-AllowlistProbeIp", default="1.1.1.1")
    parser.add_argument("-AllowlistReachabilityIp", default="")
    parser.add_argument("-ExpectAppcontainer", default="1")
    args = parser.parse_args()

    arm = args.Arm
    workspace = args.Workspace or os.environ.get("GSA_PROBE_WORKSPACE", os.getcwd())
    temp_dir = args.TempDir or os.environ.get("GSA_PROBE_TEMPDIR", "")
    expect_appcontainer = str(args.ExpectAppcontainer).strip().lower() in (
        "1",
        "true",
        "yes",
    )

    # Started marker: distinguishes "child never ran the script" from
    # "script crashed mid-run" when stdout is not capturable.
    try:
        with open(os.path.join(workspace, ".probe-started"), "w", encoding="utf-8") as fh:
            fh.write("started\n")
    except OSError:
        pass
    # Debug dumps (AppContainer children have no captured stdout).
    try:
        with open(os.path.join(workspace, ".whoami-groups.txt"), "w", encoding="utf-8") as fh:
            fh.write(_whoami(["/groups"], workspace))
        with open(os.path.join(workspace, ".whoami-priv.txt"), "w", encoding="utf-8") as fh:
            fh.write(_whoami(["/priv"], workspace))
        with open(os.path.join(workspace, ".probe-env.txt"), "w", encoding="utf-8") as fh:
            fh.write(
                "\n".join(
                    f"{k}={os.environ.get(k, '')}"
                    for k in ("TEMP", "TMP", "USERPROFILE", "LOCALAPPDATA", "APPDATA")
                )
            )
    except OSError:
        pass

    checks: dict[str, bool] = {}
    failures = 0
    debug_path = os.path.join(
        workspace, f".enforcement-probe-{arm}.debug.log"
    )

    def dbg(line: str) -> None:
        try:
            with open(debug_path, "a", encoding="utf-8") as fh:
                fh.write(line + "\n")
        except OSError:
            pass

    def check(name: str, condition: bool) -> None:
        nonlocal failures
        checks[name] = bool(condition)
        dbg(f"{'PASS' if condition else 'FAIL'}  {name}")
        print(f"{'PASS' if condition else 'FAIL'}  {name}", flush=True)
        if not condition:
            failures += 1

    wf12_timings: dict[str, int] = {}
    check("workspace_writable", path_write_succeeded(workspace))

    if arm == "control":
        os_target = os.path.join(os.environ.get("SystemRoot", "C:\\WINDOWS"), "Temp")
        if not os.path.isdir(os_target):
            os_target = os.environ.get("ProgramData", "C:\\ProgramData")
        check("os_writable", path_write_succeeded(os_target))

    elif arm == "non-admin":
        is_admin = _is_admin(workspace)
        check("non_admin", not is_admin)

        program_files = os.environ.get("ProgramFiles", "C:\\Program Files")
        local_appdata = os.environ.get("LOCALAPPDATA", "")
        pf_blocked = path_write_blocked(program_files)
        check(
            "token_virtualization_disabled",
            pf_blocked and not virtual_store_has_probe(local_appdata),
        )
        check("program_files_write_blocked", pf_blocked)
        system_root = os.environ.get("SystemRoot", "C:\\WINDOWS")
        check(
            "system32_write_blocked",
            path_write_blocked(os.path.join(system_root, "System32")),
        )
        check("hklm_write_blocked", registry_write_blocked(winreg.HKEY_LOCAL_MACHINE, r"SOFTWARE"))

        enabled = set(_enabled_privileges(workspace))
        check("privileges_removed", not bool(enabled & RESTRICTED_PRIVILEGES))

        check("runas_blocked", uac_hardened() and not is_admin)
        check("temp_write_succeeded", path_write_succeeded(os.environ.get("TEMP", "")))
        check("home_write_succeeded", path_write_succeeded(os.environ.get("USERPROFILE", "")))

    elif arm == "high-nist":
        is_admin = _is_admin(workspace)
        check("non_admin", not is_admin)

        program_files = os.environ.get("ProgramFiles", "C:\\Program Files")
        local_appdata = os.environ.get("LOCALAPPDATA", "")
        pf_blocked = path_write_blocked(program_files)
        check(
            "token_virtualization_disabled",
            pf_blocked and not virtual_store_has_probe(local_appdata),
        )
        check("program_files_write_blocked", pf_blocked)
        system_root = os.environ.get("SystemRoot", "C:\\WINDOWS")
        check(
            "system32_write_blocked",
            path_write_blocked(os.path.join(system_root, "System32")),
        )
        check("hklm_write_blocked", registry_write_blocked(winreg.HKEY_LOCAL_MACHINE, r"SOFTWARE"))

        enabled = set(_enabled_privileges(workspace))
        check("privileges_removed", not bool(enabled & RESTRICTED_PRIVILEGES))
        check("runas_blocked", uac_hardened() and not is_admin)

        programdata = os.environ.get("ProgramData", "C:\\ProgramData")
        check("programdata_write_blocked", path_write_blocked(programdata))
        drive_root = os.path.splitdrive(system_root)[0] + "\\"
        check("drive_root_write_blocked", path_write_blocked(drive_root))
        check("hkcufrozen", registry_write_blocked(winreg.HKEY_CURRENT_USER, r"SOFTWARE"))
        check("home_frozen", path_write_blocked(os.environ.get("USERPROFILE", "")))
        # AppContainer children get LOCALAPPDATA rewritten by the OS to the
        # package's own AC dir (writable by design), so assert against the
        # real user appdata path under the frozen profile instead.
        user_profile = os.environ.get("USERPROFILE", "")
        check(
            "appdata_frozen",
            path_write_blocked(os.environ.get("APPDATA", ""))
            and path_write_blocked(os.path.join(user_profile, "AppData", "Local")),
        )

        # Windows rewrites TEMP/TMP for AppContainer children to the
        # package-scoped AC\Temp (under AppData\Local\Packages\<pkg>), so the
        # sandbox-configured workspace\.tmp is not what the child sees.  Both
        # locations are contained; require one of them plus writability.
        expected_tmp = temp_dir or os.path.join(workspace, ".tmp")
        temp = os.environ.get("TEMP", "")
        tmpv = os.environ.get("TMP", "")
        tmp_contained = (
            temp.startswith(expected_tmp) and tmpv.startswith(expected_tmp)
        ) or (
            "AppData\\Local\\Packages" in temp
            and "\\AC\\Temp" in temp
            and temp == tmpv
        )
        # Windows may point TEMP at a not-yet-created package AC\Temp;
        # ensure it exists before the writability probe.
        temp_usable = False
        if tmp_contained:
            try:
                os.makedirs(temp, exist_ok=True)
                temp_usable = path_write_succeeded(temp)
            except OSError:
                temp_usable = False
        check(
            "temp_redirected",
            tmp_contained and temp_usable,
        )

        # Token facts (LOW integrity / AppContainer SID) cannot be observed
        # from inside an AppContainer child: ctypes fails to initialize
        # (libffi DLL init) and whoami.exe does not run there either.  Use
        # behavioral proxies verified against the parent-side sandbox
        # observation (which reads the real token via P/Invoke):
        #   - LOW IL: Medium-labeled C:\workspace is write-blocked while the
        #     Low-labeled arm workspace is writable.
        #   - AppContainer: the OS rewrites LOCALAPPDATA to
        #     ...\Packages\<app>\AC only for AppContainer children.
        ws_root = os.path.splitdrive(workspace)[0] + "\\workspace"
        check(
            "low_integrity",
            path_write_blocked(ws_root) and path_write_succeeded(workspace),
        )
        if expect_appcontainer:
            check(
                "appcontainer_token",
                "AppData\\Local\\Packages" in os.environ.get("LOCALAPPDATA", "")
                and "\\AC" in os.environ.get("LOCALAPPDATA", ""),
            )
        else:
            # mode=disabled (--no-appcontainer, 2026-09-03 ruling for orz.exe
            # loader compatibility): the wall keeps non-admin + LOW IL + Job +
            # TEMP redirect + egress allowlist without the AppContainer layer.
            # LOCALAPPDATA is not package-rewritten, so the AppContainer child
            # proxy must not be asserted (mirrors the parent-side observation,
            # which discards appcontainer_token when appcontainer=False).
            dbg("SKIP appcontainer_token (mode=disabled / no-appcontainer)")
        # job_object_assigned is asserted by the parent-side sandbox
        # observation (IsProcessInJob on the child handle): AppContainer
        # children cannot observe job membership without ctypes, and
        # _ctypes fails to initialize inside AppContainer (libffi DLL init).

        blocked_probe_ip = args.AllowlistProbeIp
        if args.AllowlistReachabilityIp and args.AllowlistReachabilityIp == args.AllowlistProbeIp:
            for candidate in ("8.8.8.8", "1.1.1.1"):
                if candidate != args.AllowlistReachabilityIp:
                    blocked_probe_ip = candidate
                    break
        # TER T2.2 (W-F12)：墙内 egress 判定附时延（ms）——外部目标
        # ≤1.5s 可判定失败、allowlist ≤1.5s 可达，作为验收线证据。
        _t0 = time.monotonic()
        _blocked = tcp_blocked(blocked_probe_ip, 443)
        wf12_timings["network_blocked_ms"] = int((time.monotonic() - _t0) * 1000)
        print(
            f"WF12 network_blocked_ms={wf12_timings['network_blocked_ms']}",
            flush=True,
        )
        check("network_blocked", _blocked)
        if args.AllowlistReachabilityIp:
            _t0 = time.monotonic()
            _reachable = not tcp_blocked(args.AllowlistReachabilityIp, 443)
            wf12_timings["allowlist_reachable_ms"] = int((time.monotonic() - _t0) * 1000)
            print(
                f"WF12 allowlist_reachable_ms={wf12_timings['allowlist_reachable_ms']}",
                flush=True,
            )
            check("allowlist_reachable", _reachable)
        _t0 = time.monotonic()
        _meta_blocked = tcp_blocked("169.254.169.254", 80)
        wf12_timings["metadata_blocked_ms"] = int((time.monotonic() - _t0) * 1000)
        print(
            f"WF12 metadata_blocked_ms={wf12_timings['metadata_blocked_ms']}",
            flush=True,
        )
        check("metadata_blocked", _meta_blocked)

        leftover = []
        try:
            for name in os.listdir(workspace):
                if name.startswith(".p2-probe-"):
                    leftover.append(name)
        except OSError:
            pass
        check("probe_file_cleaned", not leftover)

    if args.ResultPath:
        result = {
            "wf12_timings_ms": wf12_timings,
            "arm": arm,
            "workspace": workspace,
            "appcontainer_expected": expect_appcontainer,
            "checks": checks,
            "passed": sum(1 for v in checks.values() if v),
            "failed": failures,
            "ran_at": _now_iso(),
        }
        with open(args.ResultPath, "w", encoding="utf-8") as fh:
            json.dump(result, fh, ensure_ascii=False, indent=2)

    if failures:
        print(f"enforcement-probe: {arm} FAILED ({failures} assertions failed)", flush=True)
        return 1
    print(f"enforcement-probe: {arm} OK", flush=True)
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except SystemExit:
        raise
    except Exception:
        tb = traceback.format_exc()
        ws = None
        try:
            idx = sys.argv.index("-Workspace")
            ws = sys.argv[idx + 1]
        except (ValueError, IndexError):
            ws = os.environ.get("GSA_PROBE_WORKSPACE")
        candidates = [
            ws,
            os.environ.get("GSA_PROBE_WORKSPACE"),
            os.getcwd(),
            os.environ.get("TEMP"),
        ]
        for cand in candidates:
            if not cand:
                continue
            try:
                with open(
                    os.path.join(cand, ".enforcement-probe-crash.log"),
                    "a",
                    encoding="utf-8",
                ) as fh:
                    fh.write(tb + "\n")
            except OSError:
                pass
        sys.stderr.write(tb)
        sys.exit(1)
