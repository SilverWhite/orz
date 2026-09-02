<#
    S4 round 2 (SYSTEM task):
      1. Sync sandbox + hardening + probe runner into the VM.
      2. Fix the AgentUser profile: correct ProfileImagePath to C:\Users\
         AgentUser, create NTUSER.DAT (LoadUserProfile + reg save fallback),
         remove the wrong-path deny ACLs left by the SYSTEM-profile-root bug.
      3. Re-apply the high-nist template (correct profile root now).
      4. Run the high-nist enforcement probe.
    Host-side elevated.  ASCII only.
#>
[CmdletBinding()]
param(
    [string]$OutFile = 'D:\CLI\_windows_high_nist\vm-arms-round2-result.txt'
)

$ErrorActionPreference = 'Stop'
$lines = New-Object System.Collections.Generic.List[string]
$runner = 'D:\CLI\scripts\s4_vm_run_elev.ps1'

try {
    $sec = ConvertTo-SecureString '123456' -AsPlainText -Force
    $cred = New-Object System.Management.Automation.PSCredential('HL', $sec)
    $deadline = (Get-Date).AddSeconds(240)
    $up = $false
    while ((Get-Date) -lt $deadline) {
        try {
            $t = New-PSSession -VMName 'win-s4' -Credential $cred -ErrorAction Stop
            Remove-PSSession -Session $t -ErrorAction SilentlyContinue
            $up = $true
            break
        } catch {
            Start-Sleep -Seconds 10
        }
    }
    if (-not $up) {
        throw 'win-s4 not reachable within 240s'
    }

    $sess = New-PSSession -VMName 'win-s4' -Credential $cred
    Copy-Item -LiteralPath 'D:\CLI\assurance\windows_sandbox.py' `
        -Destination 'C:\s4\assurance\windows_sandbox.py' -ToSession $sess -Force
    Copy-Item -LiteralPath 'D:\CLI\_windows_high_nist\hardening\apply_hardening.ps1' `
        -Destination 'C:\s4\_windows_high_nist\hardening\apply_hardening.ps1' -ToSession $sess -Force
    Copy-Item -LiteralPath 'D:\CLI\_windows_high_nist\run\run_enforcement_probe.ps1' `
        -Destination 'C:\s4\_windows_high_nist\run\run_enforcement_probe.ps1' -ToSession $sess -Force
    $srcHash = (Get-FileHash -LiteralPath 'D:\CLI\assurance\windows_sandbox.py' -Algorithm SHA256).Hash
    $dstHash = Invoke-Command -Session $sess -ScriptBlock {
        (Get-FileHash -LiteralPath 'C:\s4\assurance\windows_sandbox.py' -Algorithm SHA256).Hash
    }
    $lines.Add("SYNC_WINDOWS_SANDBOX=$($srcHash -eq $dstHash)")
    Remove-PSSession -Session $sess -ErrorAction SilentlyContinue
} catch {
    $lines.Add("SYNC_ERR: $($_.Exception.Message)")
    $lines | Set-Content -LiteralPath $OutFile -Encoding utf8
    exit 1
}

$body = @'
$env:PYTHONIOENCODING = 'utf-8'
$env:Path = [Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [Environment]::GetEnvironmentVariable('Path','User')
$env:PYTHONPATH = 'C:\s4'
$py = 'C:\Program Files\Python312\python.exe'
$hard = 'C:\s4\_windows_high_nist\hardening\apply_hardening.ps1'
$run = 'C:\s4\_windows_high_nist\run\run_enforcement_probe.ps1'
$agentSid = 'S-1-5-21-3636881053-4261193759-2698424706-1002'
New-Item -ItemType Directory -Path 'C:\workspace\high-nist' -Force | Out-Null

# --- profile fix ---
$profCode = @"
import ctypes, os, subprocess
from ctypes import wintypes
advapi32 = ctypes.WinDLL('advapi32', use_last_error=True)
userenv = ctypes.WinDLL('userenv', use_last_error=True)
advapi32.LogonUserW.argtypes = [wintypes.LPCWSTR, wintypes.LPCWSTR, wintypes.LPCWSTR, wintypes.DWORD, wintypes.DWORD, ctypes.POINTER(wintypes.HANDLE)]
advapi32.LogonUserW.restype = wintypes.BOOL
class PI(ctypes.Structure):
    _fields_ = [('dwSize', wintypes.DWORD), ('dwFlags', wintypes.DWORD), ('lpUserName', wintypes.LPWSTR), ('lpProfilePath', wintypes.LPWSTR), ('lpDefaultPath', wintypes.LPWSTR), ('lpServerName', wintypes.LPWSTR), ('lpPolicyPath', wintypes.LPWSTR), ('hProfile', wintypes.HANDLE)]
userenv.LoadUserProfileW.argtypes = [wintypes.HANDLE, ctypes.POINTER(PI)]
userenv.LoadUserProfileW.restype = wintypes.BOOL
userenv.UnloadUserProfile.argtypes = [wintypes.HANDLE, wintypes.HANDLE]
userenv.UnloadUserProfile.restype = wintypes.BOOL
tok = wintypes.HANDLE()
ok = advapi32.LogonUserW('AgentUser', 'DESKTOP-QMAPMFH', 'S4Run!2026', 4, 0, ctypes.byref(tok))
print('PROF_LOGON', ok, ctypes.get_last_error())
info = PI()
info.dwSize = ctypes.sizeof(PI)
info.lpUserName = wintypes.LPWSTR('AgentUser')
okl = userenv.LoadUserProfileW(tok, ctypes.byref(info))
print('PROF_LOAD', okl, ctypes.get_last_error(), 'hProfile', info.hProfile)
ntuser = r'C:\Users\AgentUser\NTUSER.DAT'
print('NTUSER_EXISTS', os.path.exists(ntuser))
if okl and info.hProfile:
    if not os.path.exists(ntuser):
        r = subprocess.run(['reg.exe', 'save', 'HKU\\\\' + 'S-1-5-21-3636881053-4261193759-2698424706-1002', ntuser, '/y'], capture_output=True)
        print('REG_SAVE', r.returncode, r.stdout.decode('utf-8', 'replace').strip(), r.stderr.decode('utf-8', 'replace').strip())
    userenv.UnloadUserProfile(tok, info.hProfile)
print('NTUSER_EXISTS2', os.path.exists(ntuser))
print('PROF_DONE')
"@
Set-Content -LiteralPath 'C:\s4\tools\mkprofile2.py' -Value $profCode -Encoding utf8
& $py 'C:\s4\tools\mkprofile2.py' 2>&1 | ForEach-Object { Write-Output "PROF: $_" }

# fix ProfileImagePath if needed
$profKey = "HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion\ProfileList\$agentSid"
& reg.exe query $profKey /v ProfileImagePath 2>&1 | ForEach-Object { Write-Output "PIP: $_" }
& reg.exe add $profKey /v ProfileImagePath /t REG_EXPAND_SZ /d 'C:\Users\AgentUser' /f 2>&1 | ForEach-Object { Write-Output "PIP_SET: $_" }

# remove wrong-path deny ACLs from the SYSTEM-profile-root bug
& icacls.exe 'C:\WINDOWS\system32\config\AgentUser' /remove:d 'AgentUser' /T /C 2>&1 | Out-Null
Write-Output "WRONGPATH_CLEAN_EXIT=$LASTEXITCODE"

# --- re-apply high-nist (correct profile root now) ---
Write-Output '===== APPLY HIGH-NIST ====='
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File $hard -Arm high-nist -WorkspaceRoot C:\workspace 2>&1 | ForEach-Object { Write-Output "HNH: $_" }
Write-Output "HNH_EXIT=$LASTEXITCODE"

# --- high-nist probe ---
Write-Output '===== HIGH-NIST PROBE ====='
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File $run -Arm high-nist -Workspace C:\workspace\high-nist -ResultPath C:\workspace\high-nist\enforcement-probe-high-nist.json -TimeoutSeconds 180 2>&1 | ForEach-Object { Write-Output "HN: $_" }
Write-Output "HN_PROBE_EXIT=$LASTEXITCODE"
$obsHn = 'C:\workspace\high-nist\windows-native-run-observation-high-nist.json'
if (Test-Path -LiteralPath $obsHn) {
    $o = Get-Content -LiteralPath $obsHn -Raw | ConvertFrom-Json
    Write-Output "HN_OBS outcome=$($o.outcome) exit=$($o.process.exit_code)"
    if ($o.diagnostics) { $o.diagnostics | ForEach-Object { Write-Output "HN_DIAG: $_" } }
    if ($o.checks) { $o.checks.PSObject.Properties | ForEach-Object { Write-Output "HN_CHECK $($_.Name)=$($_.Value)" } }
}
if (Test-Path -LiteralPath 'C:\workspace\high-nist\enforcement-probe-high-nist.json') {
    Get-Content -LiteralPath 'C:\workspace\high-nist\enforcement-probe-high-nist.json' -Raw
}
'@

$bodyFile = 'D:\CLI\_windows_high_nist\job-arms-round2-body.ps1'
Set-Content -LiteralPath $bodyFile -Value $body -Encoding utf8
try {
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $runner `
        -JobBodyFile $bodyFile -OutFile $OutFile -TimeoutSeconds 700
    if ($LASTEXITCODE -ne 0) { throw "arms-round2 job failed (exit $LASTEXITCODE)" }
}
catch {
    $lines.Add("ERROR: $($_.Exception.Message)")
    $lines | Set-Content -LiteralPath $OutFile -Encoding utf8
}
