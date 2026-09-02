$env:Path = [Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [Environment]::GetEnvironmentVariable('Path','User')
$env:PYTHONPATH = 'C:\s4'
$py = 'C:\Program Files\Python312\python.exe'
$pw = 'S4Run!2026'

if (-not (Get-LocalUser -Name 'AgentUser' -ErrorAction SilentlyContinue)) {
    & net.exe user AgentUser $pw /add 2>&1 | ForEach-Object { Write-Output "NET_CREATE: $_" }
    Write-Output "NET_CREATE_EXIT=$LASTEXITCODE"
}
& net.exe user AgentUser $pw 2>&1 | ForEach-Object { Write-Output "NET_PW: $_" }
Write-Output "NET_PW_EXIT=$LASTEXITCODE"

[Environment]::SetEnvironmentVariable('ORZ_WINDOWS_RUN_USER', 'AgentUser', 'Machine')
[Environment]::SetEnvironmentVariable('ORZ_WINDOWS_RUN_USER_PASSWORD', $pw, 'Machine')
[Environment]::SetEnvironmentVariable('ORZ_WINDOWS_RUN_USER_DOMAIN', $env:COMPUTERNAME, 'Machine')
Write-Output "ENV_USER=$([Environment]::GetEnvironmentVariable('ORZ_WINDOWS_RUN_USER','Machine'))"
Write-Output "ENV_DOMAIN=$([Environment]::GetEnvironmentVariable('ORZ_WINDOWS_RUN_USER_DOMAIN','Machine'))"

$cfg = 'C:\s4\tools\secpol.cfg'
$sdb = 'C:\s4\tools\secpol.sdb'
Remove-Item -LiteralPath $cfg,$sdb -Force -ErrorAction SilentlyContinue
& secedit.exe /export /cfg $cfg /areas USER_RIGHTS 2>&1 | ForEach-Object { Write-Output "SE_EXPORT: $_" }
$u = [ADSI]"WinNT://$env:COMPUTERNAME/AgentUser"
$sidBytes = $u.objectSID[0]
$sid = [System.Security.Principal.SecurityIdentifier]::new($sidBytes, 0).Value
Write-Output "AGENTUSER_SID=$sid"
$cfgLines = Get-Content -LiteralPath $cfg
$outLines = New-Object System.Collections.Generic.List[string]
$patched = $false
foreach ($line in $cfgLines) {
    if ($line -match '^SeBatchLogonRight\s*=') {
        $outLines.Add($line.TrimEnd() + ",*$sid")
        $patched = $true
    }
    else {
        $outLines.Add($line)
    }
}
if (-not $patched) {
    $outLines.Add("SeBatchLogonRight = *$sid")
}
$outLines | Set-Content -LiteralPath $cfg -Encoding ascii
& secedit.exe /configure /db $sdb /cfg $cfg /areas USER_RIGHTS 2>&1 | ForEach-Object { Write-Output "SE_CONF: $_" }
Write-Output "SECEDIT_EXIT=$LASTEXITCODE"
$env:GSA_AGENT_SID = $sid

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
sid = os.environ.get('GSA_AGENT_SID', '')
tok = wintypes.HANDLE()
ok = advapi32.LogonUserW('AgentUser', os.environ.get('COMPUTERNAME', ''), 'S4Run!2026', 4, 0, ctypes.byref(tok))
print('PROF_LOGON', ok, ctypes.get_last_error())
info = PI()
info.dwSize = ctypes.sizeof(PI)
info.lpUserName = wintypes.LPWSTR('AgentUser')
okl = userenv.LoadUserProfileW(tok, ctypes.byref(info))
print('PROF_LOAD', okl, ctypes.get_last_error(), 'hProfile', info.hProfile)
ntuser = r'C:\Users\AgentUser\NTUSER.DAT'
print('NTUSER_EXISTS1', os.path.exists(ntuser))
if okl and info.hProfile:
    if not os.path.exists(ntuser) and sid:
        r = subprocess.run(['reg.exe', 'save', 'HKU\\' + sid, ntuser, '/y'], capture_output=True)
        print('REG_SAVE', r.returncode, r.stdout.decode('utf-8', 'replace').strip(), r.stderr.decode('utf-8', 'replace').strip())
    userenv.UnloadUserProfile(tok, info.hProfile)
print('NTUSER_EXISTS2', os.path.exists(ntuser))
print('PROF_DONE')
"@
Set-Content -LiteralPath 'C:\s4\tools\mkprofile_formal.py' -Value $profCode -Encoding utf8
& $py 'C:\s4\tools\mkprofile_formal.py' 2>&1 | ForEach-Object { Write-Output "PROF: $_" }

$nt = 'C:\Users\AgentUser\NTUSER.DAT'
Write-Output "NTUSER_FINAL=$(Test-Path -LiteralPath $nt)"
$profKey = "HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion\ProfileList\$sid"
& reg.exe add $profKey /v ProfileImagePath /t REG_EXPAND_SZ /d 'C:\Users\AgentUser' /f 2>&1 | ForEach-Object { Write-Output "PIP: $_" }
& icacls.exe 'C:\Users\AgentUser' /grant 'AgentUser:(OI)(CI)(M)' /T 2>&1 | Out-Null
Write-Output "PROF_ICACLS_EXIT=$LASTEXITCODE"
& icacls.exe 'C:\WINDOWS\system32\config\AgentUser' /remove:d 'AgentUser' /T /C 2>&1 | Out-Null
Write-Output "WRONGPATH_CLEAN_EXIT=$LASTEXITCODE"

if (-not $sid) {
    Write-Output 'SETUP_SID_MISSING'
    exit 1
}
if (-not (Test-Path -LiteralPath $nt)) {
    Write-Output 'SETUP_NTUSER_MISSING'
    exit 1
}
Write-Output 'SETUP_OK'
exit 0
