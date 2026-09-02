$env:Path = [Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [Environment]::GetEnvironmentVariable('Path','User')
$env:PYTHONPATH = 'C:\s4'
$ErrorActionPreference = 'Continue'
$root = 'C:\workspace\wrapup'
$py = 'C:\Program Files\Python312\python.exe'
$probeRun = 'C:\s4\_windows_high_nist\run\run_enforcement_probe.ps1'
$sandboxCli = 'C:\s4\scripts\run_windows_native_sandbox_command.py'
$credExe = 'C:\s4\tools\cred_probe.exe'
New-Item -ItemType Directory -Path $root -Force | Out-Null
$summary = [ordered]@{
    ran_at = (Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ')
    probe1 = $null
    probe2 = $null
    hive_5023_1 = $false
    hive_5023_2 = $false
    cred_nonadmin = $null
    cred_highnist = $null
    hive_mounted_post = $false
    hive_locked_post = $false
    errors = New-Object System.Collections.ArrayList
}
$u = [ADSI]"WinNT://$env:COMPUTERNAME/AgentUser"
$sidBytes = $u.objectSID[0]
$sid = [System.Security.Principal.SecurityIdentifier]::new($sidBytes, 0).Value
# 5023 wrap-up gate: after apply's HKCU-freeze reg load/unload cycle the
# profile service rejects LoadUserProfileW with WinError 5023 for several
# minutes.  Wait (SYSTEM-side, up to 600s) until the AgentUser profile
# loads and unloads cleanly before running the first sandbox probe.
$gateCode = @"
import ctypes, time, sys
from ctypes import wintypes
adv = ctypes.WinDLL('advapi32', use_last_error=True)
ue = ctypes.WinDLL('userenv', use_last_error=True)
adv.LogonUserW.argtypes = [wintypes.LPCWSTR, wintypes.LPCWSTR, wintypes.LPCWSTR, wintypes.DWORD, wintypes.DWORD, ctypes.POINTER(wintypes.HANDLE)]
adv.LogonUserW.restype = wintypes.BOOL
class PI(ctypes.Structure):
    _fields_ = [('dwSize', wintypes.DWORD), ('dwFlags', wintypes.DWORD), ('lpUserName', wintypes.LPWSTR), ('lpProfilePath', wintypes.LPWSTR), ('lpDefaultPath', wintypes.LPWSTR), ('lpServerName', wintypes.LPWSTR), ('lpPolicyPath', wintypes.LPWSTR), ('hProfile', wintypes.HANDLE)]
ue.LoadUserProfileW.argtypes = [wintypes.HANDLE, ctypes.POINTER(PI)]
ue.LoadUserProfileW.restype = wintypes.BOOL
ue.UnloadUserProfile.argtypes = [wintypes.HANDLE, wintypes.HANDLE]
ue.UnloadUserProfile.restype = wintypes.BOOL
tok = wintypes.HANDLE()
ok = adv.LogonUserW('AgentUser', '$env:COMPUTERNAME', 'S4Run!2026', 4, 0, ctypes.byref(tok))
if not ok:
    print('GATE_LOGON_FAIL', ctypes.get_last_error())
    sys.exit(1)
deadline = time.time() + 600
attempt = 0
while time.time() < deadline:
    attempt += 1
    info = PI()
    info.dwSize = ctypes.sizeof(PI)
    info.lpUserName = wintypes.LPWSTR('AgentUser')
    r = ue.LoadUserProfileW(tok, ctypes.byref(info))
    if r:
        ue.UnloadUserProfile(tok, info.hProfile)
        print('GATE_READY attempt', attempt, 'elapsed', round(time.time() - (deadline - 600), 1))
        sys.exit(0)
    err = ctypes.get_last_error()
    if attempt <= 3 or attempt % 5 == 0:
        print('GATE_WAIT attempt', attempt, 'err', err)
    time.sleep(10)
print('GATE_TIMEOUT')
sys.exit(1)
"@
Set-Content -LiteralPath (Join-Path $root 'gate_profile_ready.py') -Value $gateCode -Encoding utf8
Write-Output '===== WRAPUP HIVE READY GATE (5023 cooldown) ====='
& $py (Join-Path $root 'gate_profile_ready.py') 2>&1 | ForEach-Object { Write-Output "GATE: $_" }
$gateExit = $LASTEXITCODE
Write-Output "GATE_EXIT=$gateExit"
if ($gateExit -ne 0) {
    [void]$summary.errors.Add('hive readiness gate timed out (LoadUserProfileW 5023 not clearing)')
}
function Run-WrapupProbe {
    param([string]$Tag)
    $wsDir = Join-Path $root ("probe-ws-$Tag")
    New-Item -ItemType Directory -Path $wsDir -Force | Out-Null
    # ResultPath must live inside the sandbox workdir: the AppContainer
    # child only has write access to the workspace it was granted.
    $resPath = Join-Path $wsDir 'enforcement-probe-high-nist.json'
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $probeRun -Arm high-nist -Workspace $wsDir -ResultPath $resPath -TimeoutSeconds 200 2>&1 | ForEach-Object { Write-Output "PROBE$Tag`: $_" }
    $pExit = $LASTEXITCODE
    if (Test-Path -LiteralPath $resPath) {
        Copy-Item -LiteralPath $resPath -Destination (Join-Path $root ("enforcement-probe-wrapup-$Tag.json")) -Force
    }
    $obsSrc = Join-Path $wsDir 'windows-native-run-observation-high-nist.json'
    $obsCopy = Join-Path $root ("windows-native-run-observation-wrapup-$Tag.json")
    if (Test-Path -LiteralPath $obsSrc) {
        Copy-Item -LiteralPath $obsSrc -Destination $obsCopy -Force
    }
    $hiveHit = $false
    if (Test-Path -LiteralPath $obsSrc) {
        $o = Get-Content -LiteralPath $obsSrc -Raw -Encoding UTF8 | ConvertFrom-Json
        if ($o.diagnostics) {
            $diagShown = 0
            foreach ($d in $o.diagnostics) {
                if ($d -match '5023|LoadUserProfileW.*failed') { $hiveHit = $true }
                if ($diagShown -lt 25) {
                    Write-Output "PROBE${Tag}_DIAG: $d"
                    $diagShown++
                }
            }
        }
    }
    return [ordered]@{ tag = $Tag; probe_exit = $pExit; obs_copied = (Test-Path -LiteralPath $obsCopy); hive_5023 = $hiveHit }
}
Write-Output '===== WRAPUP PROBE 1 (immediate after apply) ====='
$summary.probe1 = Run-WrapupProbe -Tag '1'
$summary.hive_5023_1 = [bool]$summary.probe1.hive_5023
Write-Output "PROBE1 exit=$($summary.probe1.probe_exit) hive5023=$($summary.hive_5023_1)"
Write-Output '===== WRAPUP PROBE 2 (stability) ====='
$summary.probe2 = Run-WrapupProbe -Tag '2'
$summary.hive_5023_2 = [bool]$summary.probe2.hive_5023
Write-Output "PROBE2 exit=$($summary.probe2.probe_exit) hive5023=$($summary.hive_5023_2)"
if ($summary.probe1.probe_exit -ne 0 -or $summary.probe2.probe_exit -ne 0) {
    [void]$summary.errors.Add('enforcement probe failed after apply')
}
# 5023 wrap-up note: probes stay compliant (exit 0, wall assertions pass)
# even when the sandbox-side LoadUserProfileW logs a 5023 diagnostic; the
# profile-service state behind it is registered as a known gap (real HKCU
# mapping needs a dedicated SYSTEM job per sandbox run or a kept-loaded
# profile).  Recorded, not fatal.
$summary.hive_5023_note = ($summary.hive_5023_1 -or $summary.hive_5023_2)
if ($summary.hive_5023_note) {
    Write-Output 'HIVE_5023_NOTE=present (compliant; registered gap)'
}
# CredReadW inside sandbox with the NATIVE cred_probe.exe (python ctypes is
# not usable inside an AppContainer child - _ctypes/libffi DLL init fails;
# the native exe mirrors orz's own CredReadW path).
$credStates = @(
    @{ key = 'nonadmin'; arm = 'non-admin' },
    @{ key = 'highnist'; arm = 'high-nist' }
)
foreach ($cs in $credStates) {
    $wd = Join-Path $root ("cred-" + $cs.key)
    New-Item -ItemType Directory -Path $wd -Force | Out-Null
    $wdExe = Join-Path $wd 'cred_probe.exe'
    Copy-Item -LiteralPath $credExe -Destination $wdExe -Force
    if (-not (Test-Path -LiteralPath $wdExe)) {
        [void]$summary.errors.Add("cred_probe.exe missing for $($cs.key)")
        continue
    }
    $marker = Join-Path $wd '.assurance-p2-disposable.json'
    @{
        schema_version = '0.1.0-draft'
        purpose = 'windows-native-sandbox-probe'
        allow_container_write_probe = $true
    } | ConvertTo-Json | Set-Content -LiteralPath $marker -Encoding ascii
    $obs = Join-Path $root ("windows-native-run-observation-cred-" + $cs.key + '.json')
    $resIn = Join-Path $wd 'cred-result.json'
    $resOut = Join-Path $root ("cred-" + $cs.key + '.json')
    $sbxArgs = @(
        '--workspace', $wd,
        '--arm', $cs.arm,
        '--timeout', 240,
        '--output', $obs,
        '--command',
        $wdExe, $resIn
    )
    Write-Output "===== CRED SANDBOX arm=$($cs.arm) ====="
    & $py $sandboxCli @sbxArgs 2>&1 | ForEach-Object { Write-Output "CREDSBX: $_" }
    $sbxExit = $LASTEXITCODE
    Write-Output "CRED_SBX_EXIT=$sbxExit"
    if (Test-Path -LiteralPath $resIn) {
        Copy-Item -LiteralPath $resIn -Destination $resOut -Force
        $credRes = Get-Content -LiteralPath $resOut -Raw -Encoding UTF8 | ConvertFrom-Json
        if ($cs.key -eq 'nonadmin') { $summary.cred_nonadmin = $credRes }
        else { $summary.cred_highnist = $credRes }
        Write-Output "CRED_$($cs.key)_OK=$($credRes.ok)"
        if (-not $credRes.ok) {
            # Expected friction (design #8): an AppContainer token with
            # empty capabilities cannot read the user Credential Manager
            # store (CredReadW -> WinError 5).  Record it; only unexpected
            # failures fail the stage.
            if ($cs.key -eq 'highnist' -and [int]$credRes.winerror -eq 5) {
                $summary.cred_highnist_denied = $true
                Write-Output 'CRED_HIGHNIST_DENIED_EXPECTED=1'
            }
            else {
                [void]$summary.errors.Add("cred sandbox $($cs.key) read failed (winerror=$($credRes.winerror))")
            }
        }
    }
    else {
        [void]$summary.errors.Add("cred sandbox $($cs.key) result missing (sbxExit=$sbxExit)")
    }
}
# Post-wrapup hive check: every sandbox run must have unloaded the
# AgentUser profile hive before the stage ends.
$hkuOutPost = (& reg.exe query HKU 2>&1) -join "`n"
$summary.hive_mounted_post = $hkuOutPost.Contains($sid)
$hiveFilePost = 'C:\Users\AgentUser\NTUSER.DAT'
try {
    $fs = [System.IO.File]::Open($hiveFilePost, 'Open', 'ReadWrite', 'None')
    $fs.Close()
    $fs.Dispose()
    $summary.hive_locked_post = $false
}
catch {
    $summary.hive_locked_post = $true
    [void]$summary.errors.Add("NTUSER.DAT still locked after wrapup: $($_.Exception.Message)")
}
Write-Output "HIVE_POST mounted=$($summary.hive_mounted_post) locked=$($summary.hive_locked_post)"
$summary | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $root 'wrapup-summary.json') -Encoding utf8
Write-Output 'WRAPUP_MAIN_DONE'
if ($summary.errors.Count -gt 0) {
    Write-Output "WRAPUP_ERRORS=$($summary.errors.Count)"
    foreach ($e in $summary.errors) { Write-Output "WRAPUP_ERROR: $e" }
    exit 1
}
exit 0
