$env:Path = [Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [Environment]::GetEnvironmentVariable('Path','User')
$env:PYTHONPATH = 'C:\s4'
$ErrorActionPreference = 'Continue'
$root = 'C:\workspace\wrapup'
$py = 'C:\Program Files\Python312\python.exe'
$hard = 'C:\s4\_windows_high_nist\hardening\apply_hardening.ps1'
$probeRun = 'C:\s4\_windows_high_nist\run\run_enforcement_probe.ps1'
$sandboxCli = 'C:\s4\scripts\run_windows_native_sandbox_command.py'
New-Item -ItemType Directory -Path $root -Force | Out-Null
$summary = [ordered]@{
    ran_at = (Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ')
    apply_exit = -1
    applocker_effective = $false
    applocker_srpv2 = $false
    applocker_exe_rules = 0
    probe1 = $null
    probe2 = $null
    hive_5023_1 = $false
    hive_5023_2 = $false
    hive_mounted_pre_apply = $false
    cred_nonadmin = $null
    cred_highnist = $null
    hive_mounted_post = $false
    hive_locked_post = $false
    errors = New-Object System.Collections.ArrayList
}
# Pre-apply guard: the AgentUser hive must not already be mounted
$u = [ADSI]"WinNT://$env:COMPUTERNAME/AgentUser"
$sidBytes = $u.objectSID[0]
$sid = [System.Security.Principal.SecurityIdentifier]::new($sidBytes, 0).Value
$hkuOut = (& reg.exe query HKU 2>&1) -join "`n"
$summary.hive_mounted_pre_apply = $hkuOut.Contains($sid)
Write-Output "HIVE_PRE_APPLY mounted=$($summary.hive_mounted_pre_apply)"
if ($summary.hive_mounted_pre_apply) {
    [void]$summary.errors.Add('AgentUser hive mounted before apply (leaked profile load)')
}
Write-Output '===== WRAPUP APPLY HIGH-NIST ====='
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File $hard -Arm high-nist -RunUser AgentUser -WorkspaceRoot C:\workspace -DeepSeekIp 221.204.163.76 -EnableAppLocker 2>&1 | ForEach-Object { Write-Output "WRAP_APPLY: $_" }
$summary.apply_exit = $LASTEXITCODE
Write-Output "WRAP_APPLY_EXIT=$($summary.apply_exit)"
if ($summary.apply_exit -ne 0) {
    [void]$summary.errors.Add('apply high-nist failed')
}
# AppLocker re-verify
$alXml = ''
try {
    $alXml = [string](Get-AppLockerPolicy -Effective -Xml -ErrorAction Stop)
}
catch {
    [void]$summary.errors.Add('Get-AppLockerPolicy failed: ' + $_.Exception.Message)
}
if ($alXml -match '<RuleCollection Type="Exe"') {
    $summary.applocker_effective = $true
    $summary.applocker_exe_rules = @([regex]::Matches($alXml, '<(FilePublisherRule|FilePathRule|FileHashRule)>')).Count
}
& reg.exe query 'HKLM\SOFTWARE\Policies\Microsoft\Windows\SrpV2' 2>$null | Out-Null
$summary.applocker_srpv2 = ($LASTEXITCODE -eq 0)
Write-Output "APPLOCKER effective=$($summary.applocker_effective) srpv2=$($summary.applocker_srpv2) exe_rules=$($summary.applocker_exe_rules)"
if (-not $summary.applocker_effective -or -not $summary.applocker_srpv2) {
    [void]$summary.errors.Add('AppLocker verify failed')
}
function Run-WrapupProbe {
    param([string]$Tag)
    $wsDir = Join-Path $root ("probe-ws-$Tag")
    New-Item -ItemType Directory -Path $wsDir -Force | Out-Null
    $resPath = Join-Path $root ("enforcement-probe-wrapup-$Tag.json")
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $probeRun -Arm high-nist -Workspace $wsDir -ResultPath $resPath -TimeoutSeconds 200 2>&1 | ForEach-Object { Write-Output "PROBE$Tag`: $_" }
    $pExit = $LASTEXITCODE
    $obsSrc = Join-Path $wsDir 'windows-native-run-observation-high-nist.json'
    $obsCopy = Join-Path $root ("windows-native-run-observation-wrapup-$Tag.json")
    if (Test-Path -LiteralPath $obsSrc) {
        Copy-Item -LiteralPath $obsSrc -Destination $obsCopy -Force
    }
    $hiveHit = $false
    if (Test-Path -LiteralPath $obsSrc) {
        $o = Get-Content -LiteralPath $obsSrc -Raw -Encoding UTF8 | ConvertFrom-Json
        if ($o.diagnostics) {
            foreach ($d in $o.diagnostics) {
                if ($d -match '5023|LoadUserProfileW.*failed') { $hiveHit = $true }
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
if ($summary.hive_5023_1 -or $summary.hive_5023_2) {
    [void]$summary.errors.Add('LoadUserProfileW 5023 regression present')
}
# CredReadW inside sandbox: non-admin (AgentUser restricted) + high-nist
# (AgentUser + LOW IL + AppContainer)
$credPy = @"
import ctypes, json, sys
from ctypes import wintypes
TARGET = 'orz-deepseek/agent'
class FILETIME(ctypes.Structure):
    _fields_ = [('dwLowDateTime', wintypes.DWORD), ('dwHighDateTime', wintypes.DWORD)]
class CREDENTIALW(ctypes.Structure):
    _fields_ = [('Flags', wintypes.DWORD), ('Type', wintypes.DWORD), ('TargetName', wintypes.LPWSTR), ('Comment', wintypes.LPWSTR), ('LastWritten', FILETIME), ('CredentialBlobSize', wintypes.DWORD), ('CredentialBlob', ctypes.POINTER(ctypes.c_ubyte)), ('Persist', wintypes.DWORD), ('AttributeCount', wintypes.DWORD), ('Attributes', ctypes.c_void_p), ('TargetAlias', wintypes.LPWSTR), ('UserName', wintypes.LPWSTR)]
def probe():
    adv = ctypes.WinDLL('advapi32', use_last_error=True)
    adv.CredReadW.argtypes = [wintypes.LPCWSTR, wintypes.DWORD, wintypes.DWORD, ctypes.POINTER(ctypes.c_void_p)]
    adv.CredReadW.restype = wintypes.BOOL
    adv.CredFree.argtypes = [ctypes.c_void_p]
    adv.CredFree.restype = None
    ptr = ctypes.c_void_p()
    ok = adv.CredReadW(TARGET, 1, 0, ctypes.byref(ptr))
    if not ok:
        e = ctypes.get_last_error()
        return {'ok': False, 'winerror': int(e), 'detail': str(ctypes.WinError(e))}
    try:
        cred = ctypes.cast(ptr, ctypes.POINTER(CREDENTIALW)).contents
        return {'ok': True, 'blob_bytes': int(cred.CredentialBlobSize), 'user_name': str(cred.UserName or ''), 'persist': int(cred.Persist)}
    finally:
        adv.CredFree(ptr)
res = probe()
res['target'] = TARGET
out = sys.argv[1] if len(sys.argv) > 1 else 'cred-result.json'
with open(out, 'w', encoding='utf-8') as f:
    json.dump(res, f)
print('CRED_RESULT ' + json.dumps(res))
"@
Set-Content -LiteralPath (Join-Path $root 'cred_probe.py') -Value $credPy -Encoding utf8
$credStates = @(
    @{ key = 'nonadmin'; arm = 'non-admin' },
    @{ key = 'highnist'; arm = 'high-nist' }
)
foreach ($cs in $credStates) {
    $wd = Join-Path $root ("cred-" + $cs.key)
    New-Item -ItemType Directory -Path $wd -Force | Out-Null
    Copy-Item -LiteralPath (Join-Path $root 'cred_probe.py') -Destination (Join-Path $wd 'cred_probe.py') -Force
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
        $py, (Join-Path $wd 'cred_probe.py'), $resIn
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
            [void]$summary.errors.Add("cred sandbox $($cs.key) read failed")
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
