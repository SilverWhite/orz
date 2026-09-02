<#
    S4: one batch round on win-s4 (SYSTEM task).
      1. Sync windows_sandbox.py + apply_hardening.ps1 into the VM.
      2. In-VM: create AgentUser profile headlessly (LoadUserProfile), grant
         AgentUser modify on its profile, switch to the non-admin template,
         run the non-admin probe, switch back to the high-nist template,
         run the high-nist probe, and report everything.
    Host-side elevated.  ASCII only.
#>
[CmdletBinding()]
param(
    [string]$OutFile = 'D:\CLI\_windows_high_nist\vm-arms-round-result.txt'
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
New-Item -ItemType Directory -Path 'C:\workspace\non-admin' -Force | Out-Null
New-Item -ItemType Directory -Path 'C:\workspace\high-nist' -Force | Out-Null

# --- 1. create AgentUser profile headlessly (NTUSER.DAT + proper ACLs) ---
$profCode = @"
import ctypes
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
if okl and info.hProfile:
    userenv.UnloadUserProfile(tok, info.hProfile)
print('PROF_DONE')
"@
Set-Content -LiteralPath 'C:\s4\tools\mkprofile.py' -Value $profCode -Encoding utf8
& $py 'C:\s4\tools\mkprofile.py' 2>&1 | ForEach-Object { Write-Output "PROF: $_" }
if (Test-Path -LiteralPath 'C:\Users\AgentUser\NTUSER.DAT') {
    Write-Output 'PROF_NTUSER=yes'
} else {
    Write-Output 'PROF_NTUSER=no'
}
& icacls.exe 'C:\Users\AgentUser' /grant 'AgentUser:(OI)(CI)(M)' /T 2>&1 | Out-Null
Write-Output "PROF_ICACLS_EXIT=$LASTEXITCODE"

# --- 2. revert high-nist, apply non-admin template ---
Write-Output '===== REVERT HIGH-NIST ====='
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File $hard -Arm high-nist -Revert -WorkspaceRoot C:\workspace 2>&1 | ForEach-Object { Write-Output "REVHN: $_" }
Write-Output "REVHN_EXIT=$LASTEXITCODE"
Write-Output '===== APPLY NON-ADMIN ====='
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File $hard -Arm non-admin -WorkspaceRoot C:\workspace 2>&1 | ForEach-Object { Write-Output "NAH: $_" }
Write-Output "NAH_EXIT=$LASTEXITCODE"

# --- 3. non-admin probe ---
Write-Output '===== NON-ADMIN PROBE ====='
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File $run -Arm non-admin -Workspace C:\workspace\non-admin -ResultPath C:\workspace\non-admin\enforcement-probe-non-admin.json -TimeoutSeconds 180 2>&1 | ForEach-Object { Write-Output "NA: $_" }
Write-Output "NA_PROBE_EXIT=$LASTEXITCODE"
$obsNa = 'C:\workspace\non-admin\windows-native-run-observation-non-admin.json'
if (Test-Path -LiteralPath $obsNa) {
    $o = Get-Content -LiteralPath $obsNa -Raw | ConvertFrom-Json
    Write-Output "NA_OBS outcome=$($o.outcome) exit=$($o.process.exit_code)"
    if ($o.diagnostics) { $o.diagnostics | ForEach-Object { Write-Output "NA_DIAG: $_" } }
    if ($o.checks) { $o.checks.PSObject.Properties | ForEach-Object { Write-Output "NA_CHECK $($_.Name)=$($_.Value)" } }
}
if (Test-Path -LiteralPath 'C:\workspace\non-admin\enforcement-probe-non-admin.json') {
    Get-Content -LiteralPath 'C:\workspace\non-admin\enforcement-probe-non-admin.json' -Raw
}

# --- 4. revert non-admin, re-apply high-nist template ---
Write-Output '===== REVERT NON-ADMIN ====='
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File $hard -Arm non-admin -Revert -WorkspaceRoot C:\workspace 2>&1 | ForEach-Object { Write-Output "REVNA: $_" }
Write-Output "REVNA_EXIT=$LASTEXITCODE"
Write-Output '===== APPLY HIGH-NIST ====='
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File $hard -Arm high-nist -WorkspaceRoot C:\workspace 2>&1 | ForEach-Object { Write-Output "HNH: $_" }
Write-Output "HNH_EXIT=$LASTEXITCODE"

# --- 5. high-nist probe ---
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

$bodyFile = 'D:\CLI\_windows_high_nist\job-arms-round-body.ps1'
Set-Content -LiteralPath $bodyFile -Value $body -Encoding utf8
try {
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $runner `
        -JobBodyFile $bodyFile -OutFile $OutFile -TimeoutSeconds 900
    if ($LASTEXITCODE -ne 0) { throw "arms-round job failed (exit $LASTEXITCODE)" }
}
catch {
    $lines.Add("ERROR: $($_.Exception.Message)")
    $lines | Set-Content -LiteralPath $OutFile -Encoding utf8
}
