$env:Path = [Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [Environment]::GetEnvironmentVariable('Path','User')
$ErrorActionPreference = 'Continue'
Write-Output '===== HKU MOUNTS ====='
& reg.exe query HKU 2>&1 | ForEach-Object { Write-Output "HKU: $_" }
$u = [ADSI]"WinNT://$env:COMPUTERNAME/AgentUser"
$sidBytes = $u.objectSID[0]
$sid = [System.Security.Principal.SecurityIdentifier]::new($sidBytes, 0).Value
Write-Output "AGENTUSER_SID=$sid"
Write-Output '===== PROFILE LIST ====='
& reg.exe query "HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion\ProfileList\$sid" 2>&1 | ForEach-Object { Write-Output "PROF: $_" }
Write-Output '===== NTUSER.DAT LOCK CHECK ====='
$hiveFile = 'C:\Users\AgentUser\NTUSER.DAT'
Write-Output "NTUSER_EXISTS=$(Test-Path -LiteralPath $hiveFile)"
try {
    $fs = [System.IO.File]::Open($hiveFile, 'Open', 'ReadWrite', 'None')
    $fs.Close()
    $fs.Dispose()
    Write-Output 'NTUSER_LOCK=free'
}
catch {
    Write-Output "NTUSER_LOCK=locked detail=$($_.Exception.Message)"
}
Write-Output '===== LOADUSERPROFILE TEST (SYSTEM, AgentUser batch token) ====='
$py = 'C:\Program Files\Python312\python.exe'
$loadCode = @"
import ctypes, time
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
print('LUP_LOGON', ok, ctypes.get_last_error())
if ok:
    for attempt, pause in enumerate((1, 2, 4, 8, 16), start=1):
        info = PI()
        info.dwSize = ctypes.sizeof(PI)
        info.lpUserName = wintypes.LPWSTR('AgentUser')
        r = ue.LoadUserProfileW(tok, ctypes.byref(info))
        print('LUP_TRY', attempt, r, ctypes.get_last_error(), 'hProfile', info.hProfile)
        if r:
            u = ue.UnloadUserProfile(tok, info.hProfile)
            print('LUP_UNLOAD', u, ctypes.get_last_error())
            break
        time.sleep(pause)
print('LUP_DONE')
"@
Set-Content -LiteralPath 'C:\s4\tools\lup_diag.py' -Value $loadCode -Encoding utf8
& $py 'C:\s4\tools\lup_diag.py' 2>&1 | ForEach-Object { Write-Output "LUP: $_" }
Write-Output '===== PROCESSES ====='
Get-CimInstance Win32_Process -ErrorAction SilentlyContinue |
    Where-Object { $_.Name -match 'python|powershell|cmd' -and $_.CommandLine -match 's4|elev|cred|mkprofile|AgentUser|wrapup|clash' } |
    ForEach-Object { Write-Output "PROC pid=$($_.ProcessId) ppid=$($_.ParentProcessId) name=$($_.Name) cmd=$($_.CommandLine)" }
Write-Output '===== TASKS ====='
$csv = (& schtasks.exe /query /fo csv /nh 2>$null) -join "`n"
foreach ($line in ($csv -split "`n")) {
    if ($line -match '^"(s4[^"]*)"') {
        Write-Output "TASK $($Matches[1])"
    }
}
Write-Output 'HIVEDIAG_DONE'
