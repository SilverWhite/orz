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
