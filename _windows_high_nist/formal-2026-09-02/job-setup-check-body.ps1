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
