<#
    S4 setup-state check (host-side elevated): verifies inside win-s4 that
    AgentUser exists, has SeBatchLogonRight, can logon with BATCH, and that
    the profile NTUSER.DAT is present.  Writes a result file.
    ASCII only.
#>
[CmdletBinding()]
param(
    [string]$OutFile = 'D:\CLI\_windows_high_nist\formal-2026-09-02\setup-check.txt'
)

$ErrorActionPreference = 'Stop'
$vmName = 'win-s4'
$runner = 'D:\CLI\scripts\s4_vm_run_elev.ps1'
$lines = New-Object System.Collections.Generic.List[string]

try {
    $sec = ConvertTo-SecureString '123456' -AsPlainText -Force
    $cred = New-Object System.Management.Automation.PSCredential('HL', $sec)
    $deadline = (Get-Date).AddSeconds(240)
    $up = $false
    while ((Get-Date) -lt $deadline) {
        try {
            $t = New-PSSession -VMName $vmName -Credential $cred -ErrorAction Stop
            Remove-PSSession -Session $t -ErrorAction SilentlyContinue
            $up = $true
            break
        }
        catch {
            Start-Sleep -Seconds 10
        }
    }
    if (-not $up) {
        throw 'win-s4 not reachable within 240s'
    }

    $body = @'
$env:Path = [Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [Environment]::GetEnvironmentVariable('Path','User')
$env:PYTHONPATH = 'C:\s4'
$py = 'C:\Program Files\Python312\python.exe'
$cfg = 'C:\s4\tools\secpol-check.cfg'
Remove-Item -LiteralPath $cfg -Force -ErrorAction SilentlyContinue
& secedit.exe /export /cfg $cfg /areas USER_RIGHTS 2>&1 | Out-Null
$m = Select-String -LiteralPath $cfg -Pattern '^SeBatchLogonRight\s*='
if ($m) {
    Write-Output "SEBATCH_LINE=$($m.Line)"
}
else {
    Write-Output 'SEBATCH_LINE=<missing>'
}
$u = [ADSI]"WinNT://$env:COMPUTERNAME/AgentUser"
$sidBytes = $u.objectSID[0]
$sid = [System.Security.Principal.SecurityIdentifier]::new($sidBytes, 0).Value
Write-Output "AGENTUSER_SID=$sid"
$profKey = "HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion\ProfileList\$sid"
& reg.exe query $profKey /v ProfileImagePath 2>&1 | ForEach-Object { Write-Output "PIP: $_" }

$logonCode = @"
import ctypes
from ctypes import wintypes
advapi32 = ctypes.WinDLL('advapi32', use_last_error=True)
advapi32.LogonUserW.argtypes = [wintypes.LPCWSTR, wintypes.LPCWSTR, wintypes.LPCWSTR, wintypes.DWORD, wintypes.DWORD, ctypes.POINTER(wintypes.HANDLE)]
advapi32.LogonUserW.restype = wintypes.BOOL
tok = wintypes.HANDLE()
ok = advapi32.LogonUserW('AgentUser', '$env:COMPUTERNAME', 'S4Run!2026', 4, 0, ctypes.byref(tok))
print('LOGON_BATCH_OK', ok, 'err', ctypes.get_last_error())
"@
Set-Content -LiteralPath 'C:\s4\tools\logon_batch_check.py' -Value $logonCode -Encoding utf8
& $py 'C:\s4\tools\logon_batch_check.py' 2>&1 | ForEach-Object { Write-Output "LOGON: $_" }
Write-Output "NTUSER=$(Test-Path -LiteralPath 'C:\Users\AgentUser\NTUSER.DAT')"
Write-Output "PROFILE_DIR=$(Test-Path -LiteralPath 'C:\Users\AgentUser')"
Write-Output 'CHECK_DONE'
'@

    $bodyFile = 'D:\CLI\_windows_high_nist\formal-2026-09-02\job-setup-check-body.ps1'
    $resFile = 'D:\CLI\_windows_high_nist\formal-2026-09-02\job-setup-check-result.txt'
    Set-Content -LiteralPath $bodyFile -Value $body -Encoding utf8
    Remove-Item -LiteralPath $resFile -Force -ErrorAction SilentlyContinue
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $runner `
        -JobBodyFile $bodyFile -OutFile $resFile -TimeoutSeconds 300
    if ($LASTEXITCODE -ne 0) {
        throw "setup-check job runner failed (exit $LASTEXITCODE)"
    }
    foreach ($l in (Get-Content -LiteralPath $resFile -Encoding UTF8)) {
        $lines.Add($l)
    }
}
catch {
    $lines.Add("ERROR: $($_.Exception.Message)")
}
$lines | Set-Content -LiteralPath $OutFile -Encoding utf8
Write-Output "resultFile=$OutFile"
