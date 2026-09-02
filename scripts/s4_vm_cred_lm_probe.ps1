<#
    S4 credential-injection LocalMachine visibility probe (host-side elevated).

    Runs one SYSTEM job inside win-s4 that:
      1. writes a dummy generic credential (persist=LocalMachine) as SYSTEM,
      2. reads it back under SYSTEM,
      3. reads it from the non-admin AgentUser sandbox,
      4. reads it from the high-nist AppContainer sandbox,
      5. deletes the dummy credential.

    Answers whether an empty-capability AppContainer can CredReadW a
    LocalMachine-persisted credential (the zero-code injection route).
    ASCII only.
#>
[CmdletBinding()]
param(
    [string]$OutFile = 'D:\CLI\_windows_high_nist\vm-cred-lm-result.txt'
)

$ErrorActionPreference = 'Stop'
$vmName = 'win-s4'
$runner = 'D:\CLI\scripts\s4_vm_run_elev.ps1'
$exe = 'D:\CLI\_windows_high_nist\tools\cred_probe.exe'
$lines = New-Object System.Collections.Generic.List[string]

try {
    $sec = ConvertTo-SecureString '123456' -AsPlainText -Force
    $cred = New-Object System.Management.Automation.PSCredential('HL', $sec)

    # VM reachability + sync the rebuilt native probe.
    $sess = $null
    $deadline = (Get-Date).AddSeconds(240)
    while (-not $sess -and (Get-Date) -lt $deadline) {
        try {
            $sess = New-PSSession -VMName $vmName -Credential $cred -ErrorAction Stop
        }
        catch {
            Start-Sleep -Seconds 10
        }
    }
    if (-not $sess) {
        throw 'win-s4 not reachable within 240s'
    }
    try {
        Invoke-Command -Session $sess -ScriptBlock {
            New-Item -ItemType Directory -Path 'C:\s4\tools' -Force | Out-Null
        }
        Copy-Item -LiteralPath $exe -Destination 'C:\s4\tools\cred_probe.exe' -ToSession $sess -Force
        $srcHash = (Get-FileHash -LiteralPath $exe -Algorithm SHA256).Hash
        $dstHash = Invoke-Command -Session $sess -ScriptBlock {
            (Get-FileHash -LiteralPath 'C:\s4\tools\cred_probe.exe' -Algorithm SHA256).Hash
        }
        if ($srcHash -ne $dstHash) {
            throw 'cred_probe.exe sync hash mismatch'
        }
    }
    finally {
        Remove-PSSession -Session $sess -ErrorAction SilentlyContinue
    }

    $body = @'
$env:Path = [Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [Environment]::GetEnvironmentVariable('Path','User')
$env:PYTHONPATH = 'C:\s4'
$ErrorActionPreference = 'Continue'
$root = 'C:\workspace\cred-lm'
New-Item -ItemType Directory -Path $root -Force | Out-Null
$py = 'C:\Program Files\Python312\python.exe'
$sandboxCli = 'C:\s4\scripts\run_windows_native_sandbox_command.py'
$credExe = 'C:\s4\tools\cred_probe.exe'
$target = 's4-lm-visibility-test'
$errors = New-Object System.Collections.ArrayList

Write-Output '===== STEP 1: SYSTEM write + readback (LocalMachine persist) ====='
$sysCode = @"
import ctypes, json
from ctypes import wintypes
adv = ctypes.WinDLL('advapi32', use_last_error=True)
class FILETIME(ctypes.Structure):
    _fields_ = [('dwLowDateTime', wintypes.DWORD), ('dwHighDateTime', wintypes.DWORD)]
class CREDENTIALW(ctypes.Structure):
    _fields_ = [
        ('Flags', wintypes.DWORD),
        ('Type_', wintypes.DWORD),
        ('TargetName', ctypes.c_wchar_p),
        ('Comment', ctypes.c_wchar_p),
        ('LastWritten', FILETIME),
        ('CredentialBlobSize', wintypes.DWORD),
        ('CredentialBlob', ctypes.c_void_p),
        ('Persist', wintypes.DWORD),
        ('AttributeCount', wintypes.DWORD),
        ('Attributes', ctypes.c_void_p),
        ('TargetAlias', ctypes.c_wchar_p),
        ('UserName', ctypes.c_wchar_p),
    ]
adv.CredWriteW.argtypes = [ctypes.POINTER(CREDENTIALW), wintypes.DWORD]
adv.CredWriteW.restype = wintypes.BOOL
adv.CredReadW.argtypes = [ctypes.c_wchar_p, wintypes.DWORD, wintypes.DWORD, ctypes.POINTER(ctypes.POINTER(CREDENTIALW))]
adv.CredReadW.restype = wintypes.BOOL
adv.CredFree.argtypes = [ctypes.c_void_p]
target = 's4-lm-visibility-test'
blob = b'S4-LM-PROBE-V1'
res = {'target': target, 'write_ok': False, 'write_err': 0, 'read_ok': False, 'read_err': 0, 'persist': 0, 'blob_bytes': 0, 'blob_match': False}
buf = ctypes.create_string_buffer(blob)
cred = CREDENTIALW()
cred.Type_ = 1
cred.TargetName = target
cred.Comment = 's4 lm visibility probe'
cred.CredentialBlob = ctypes.cast(buf, ctypes.c_void_p)
cred.CredentialBlobSize = len(blob)
cred.Persist = 2
cred.UserName = 'S4Probe'
wok = adv.CredWriteW(ctypes.byref(cred), 0)
res['write_ok'] = bool(wok)
if not wok:
    res['write_err'] = ctypes.get_last_error()
p = ctypes.POINTER(CREDENTIALW)()
rok = adv.CredReadW(target, 1, 0, ctypes.byref(p))
if rok:
    c = p.contents
    data = ctypes.string_at(c.CredentialBlob, c.CredentialBlobSize)
    res['read_ok'] = True
    res['persist'] = int(c.Persist)
    res['blob_bytes'] = len(data)
    res['blob_match'] = data == blob
    adv.CredFree(ctypes.cast(p, ctypes.c_void_p))
else:
    res['read_err'] = ctypes.get_last_error()
with open(r'C:\workspace\cred-lm\sys-lm.json', 'w', encoding='utf-8') as f:
    json.dump(res, f)
print('SYS_RESULT ' + json.dumps(res))
"@
Set-Content -LiteralPath (Join-Path $root 'lm_sys.py') -Value $sysCode -Encoding utf8
& $py (Join-Path $root 'lm_sys.py') 2>&1 | ForEach-Object { Write-Output "SYS: $_" }

function Invoke-CredReadSandbox {
    param([string]$Arm, [string]$Label)
    $wd = Join-Path $root ("read-" + $Label)
    New-Item -ItemType Directory -Path $wd -Force | Out-Null
    $wdExe = Join-Path $wd 'cred_probe.exe'
    Copy-Item -LiteralPath $credExe -Destination $wdExe -Force
    if (-not (Test-Path -LiteralPath $wdExe)) {
        [void]$errors.Add("cred_probe.exe missing for $Label")
        return
    }
    $marker = Join-Path $wd '.assurance-p2-disposable.json'
    @{
        schema_version = '0.1.0-draft'
        purpose = 'windows-native-sandbox-probe'
        allow_container_write_probe = $true
    } | ConvertTo-Json | Set-Content -LiteralPath $marker -Encoding ascii
    $obs = Join-Path $wd 'windows-native-run-observation.json'
    $resIn = Join-Path $wd 'cred-result.json'
    $resOut = Join-Path $root ("cred-" + $Label + '.json')
    $obsOut = Join-Path $root ("obs-" + $Label + '.json')
    $sbxArgs = @(
        '--workspace', $wd,
        '--arm', $Arm,
        '--timeout', 240,
        '--output', $obs,
        '--env', "CRED_PROBE_TARGET=$target",
        '--command',
        $wdExe, $resIn
    )
    Write-Output "===== CRED READ arm=$Arm label=$Label ====="
    & $py $sandboxCli @sbxArgs 2>&1 | ForEach-Object { Write-Output "SBX[$Label]: $_" }
    $sbxExit = $LASTEXITCODE
    Write-Output "SBX_EXIT[$Label]=$sbxExit"
    if (Test-Path -LiteralPath $resIn) {
        Copy-Item -LiteralPath $resIn -Destination $resOut -Force
        if (Test-Path -LiteralPath $obs) {
            Copy-Item -LiteralPath $obs -Destination $obsOut -Force
        }
        $r = Get-Content -LiteralPath $resOut -Raw -Encoding UTF8 | ConvertFrom-Json
        Write-Output "CRED[$Label] ok=$($r.ok) winerror=$($r.winerror) persist=$($r.persist) blob_bytes=$($r.blob_bytes) target=$($r.target)"
        if ($r.ok) {
            Write-Output "CRED_$Label`_OK=True"
        }
        else {
            Write-Output "CRED_$Label`_OK=False"
        }
    }
    else {
        [void]$errors.Add("cred result missing label=$Label sbxExit=$sbxExit")
    }
}

Write-Output '===== STEP 2: non-admin AgentUser sandbox read ====='
Invoke-CredReadSandbox -Arm 'non-admin' -Label 'nonadmin'
Write-Output '===== STEP 3: high-nist AppContainer sandbox read ====='
Invoke-CredReadSandbox -Arm 'high-nist' -Label 'highnist'

Write-Output '===== STEP 4: cleanup dummy credential ====='
$cleanCode = @"
import ctypes, json
from ctypes import wintypes
adv = ctypes.WinDLL('advapi32', use_last_error=True)
adv.CredDeleteW.argtypes = [ctypes.c_wchar_p, wintypes.DWORD, wintypes.DWORD]
adv.CredDeleteW.restype = wintypes.BOOL
res = {'delete_ok': False, 'delete_err': 0}
ok = adv.CredDeleteW('s4-lm-visibility-test', 1, 0)
res['delete_ok'] = bool(ok)
if not ok:
    res['delete_err'] = ctypes.get_last_error()
with open(r'C:\workspace\cred-lm\cleanup-lm.json', 'w', encoding='utf-8') as f:
    json.dump(res, f)
print('CLEANUP ' + json.dumps(res))
"@
Set-Content -LiteralPath (Join-Path $root 'lm_cleanup.py') -Value $cleanCode -Encoding utf8
& $py (Join-Path $root 'lm_cleanup.py') 2>&1 | ForEach-Object { Write-Output "CLEAN: $_" }

if ($errors.Count -gt 0) {
    Write-Output "CRED_LM_ERRORS=$($errors.Count)"
    foreach ($e in $errors) {
        Write-Output "CRED_LM_ERROR: $e"
    }
    Write-Output 'CRED_LM_DONE'
    exit 1
}
Write-Output 'CRED_LM_ERRORS=0'
Write-Output 'CRED_LM_DONE'
exit 0
'@

    $bodyFile = 'D:\CLI\_windows_high_nist\formal-2026-09-02\job-cred-lm-body.ps1'
    $resFile = 'D:\CLI\_windows_high_nist\formal-2026-09-02\job-cred-lm-result.txt'
    Set-Content -LiteralPath $bodyFile -Value $body -Encoding utf8
    Remove-Item -LiteralPath $resFile -Force -ErrorAction SilentlyContinue
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $runner `
        -JobBodyFile $bodyFile -OutFile $resFile -TimeoutSeconds 900
    if ($LASTEXITCODE -ne 0) {
        throw "cred-lm job runner failed (exit $LASTEXITCODE)"
    }
    foreach ($l in (Get-Content -LiteralPath $resFile -Encoding UTF8)) {
        $lines.Add($l)
    }

    # Collect guest evidence JSONs.
    $evDir = 'D:\CLI\_windows_high_nist\formal-2026-09-02\evidence-cred-lm'
    New-Item -ItemType Directory -Path $evDir -Force | Out-Null
    $sess2 = New-PSSession -VMName $vmName -Credential $cred -ErrorAction Stop
    try {
        $guestFiles = @(Invoke-Command -Session $sess2 -ScriptBlock {
            Get-ChildItem -LiteralPath 'C:\workspace\cred-lm' -Filter '*.json' -File |
                ForEach-Object { $_.FullName }
        })
        foreach ($gf in $guestFiles) {
            Copy-Item -Path $gf -Destination $evDir -FromSession $sess2 -Force
            $lines.Add("EVIDENCE $gf")
        }
    }
    finally {
        Remove-PSSession -Session $sess2 -ErrorAction SilentlyContinue
    }

    $doneOk = @($lines | Where-Object { $_ -match '^CRED_LM_DONE$' }).Count -gt 0
    $errOk = @($lines | Where-Object { $_ -match '^CRED_LM_ERRORS=0$' }).Count -gt 0
    $ok = $doneOk -and $errOk
    $lines.Add("CRED_LM_OK=$ok")
}
catch {
    $lines.Add("ERROR: $($_.Exception.Message)")
    $lines.Add('CRED_LM_OK=False')
}

$lines | Set-Content -LiteralPath $OutFile -Encoding utf8
Write-Output "resultFile=$OutFile"
if (@($lines | Where-Object { $_ -match '^CRED_LM_OK=True$' }).Count -gt 0) {
    exit 0
}
exit 1
