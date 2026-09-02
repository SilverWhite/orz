<#
    S4 repro v10 (SYSTEM task): isolate why the sandbox's high-nist child
    exits 0xC0000142 while the v9 AppContainer replica exits 42.
    Variants over AppContainer + LOW IL + CPAU + EXTENDED(job+seccaps):
      A  no suspend, no pipes, cmd          (v9-style baseline, expect 42)
      B  no suspend, pipes, cmd
      C  suspend (resume), no pipes, cmd
      D  suspend + pipes, cmd
      E  suspend + pipes + kill-on-close job with limits, cmd
      F  full replica with powershell probe command
      G  no AppContainer, suspend + pipes, powershell (isolate powershell)
    Also fixes the AgentUser profile hive: reg save HKU\<SID> to
    C:\Users\AgentUser\NTUSER.DAT (ProfileImagePath already corrected).
    ASCII only.
#>
[CmdletBinding()]
param(
    [string]$OutFile = 'D:\CLI\_windows_high_nist\vm-repro-v10-result.txt'
)

$ErrorActionPreference = 'Stop'
$lines = New-Object System.Collections.Generic.List[string]

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
} catch {
    $lines.Add("ERROR: $($_.Exception.Message)")
    $lines | Set-Content -LiteralPath $OutFile -Encoding utf8
    exit 1
}

$body = @'
$env:PYTHONIOENCODING = 'utf-8'
$env:Path = [Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [Environment]::GetEnvironmentVariable('Path','User')
$env:PYTHONPATH = 'C:\s4'
$py = 'C:\Program Files\Python312\python.exe'
New-Item -ItemType Directory -Path 'C:\workspace\high-nist' -Force | Out-Null

# --- fix profile hive: reg save loaded hive -> C:\Users\AgentUser\NTUSER.DAT ---
$fixCode = @"
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
print('FIX_LOGON', ok, ctypes.get_last_error())
info = PI()
info.dwSize = ctypes.sizeof(PI)
info.lpUserName = wintypes.LPWSTR('AgentUser')
okl = userenv.LoadUserProfileW(tok, ctypes.byref(info))
print('FIX_LOAD', okl, ctypes.get_last_error(), 'hProfile', info.hProfile)
ntuser = r'C:\Users\AgentUser\NTUSER.DAT'
if okl and info.hProfile and not os.path.exists(ntuser):
    r = subprocess.run(['reg.exe', 'save', 'HKU\\S-1-5-21-3636881053-4261193759-2698424706-1002', ntuser, '/y'], capture_output=True)
    print('FIX_REG_SAVE', r.returncode, r.stdout.decode('utf-8', 'replace').strip(), r.stderr.decode('utf-8', 'replace').strip())
    userenv.UnloadUserProfile(tok, info.hProfile)
print('FIX_NTUSER', os.path.exists(ntuser))
"@
Set-Content -LiteralPath 'C:\s4\tools\fixprofile.py' -Value $fixCode -Encoding utf8
& $py 'C:\s4\tools\fixprofile.py' 2>&1 | ForEach-Object { Write-Output "FIX: $_" }

$pyCode = @"
import ctypes, os
from ctypes import wintypes

def log(*a):
    print(' '.join(str(x) for x in a), flush=True)

kernel32 = ctypes.WinDLL('kernel32', use_last_error=True)
advapi32 = ctypes.WinDLL('advapi32', use_last_error=True)
user32 = ctypes.WinDLL('user32', use_last_error=True)
userenv = ctypes.WinDLL('userenv', use_last_error=True)

EXTENDED = 0x00080000
SUSPENDED = 0x00000004
NO_WINDOW = 0x08000000
NEW_GROUP = 0x00000200
UNICODE_ENV = 0x00000400

user32.GetProcessWindowStation.argtypes = []
user32.GetProcessWindowStation.restype = wintypes.HANDLE
hwinsta = user32.GetProcessWindowStation()
kernel32.GetCurrentThreadId.argtypes = []
kernel32.GetCurrentThreadId.restype = wintypes.DWORD
user32.GetThreadDesktop.argtypes = [wintypes.DWORD]
user32.GetThreadDesktop.restype = wintypes.HANDLE
hdesk = user32.GetThreadDesktop(kernel32.GetCurrentThreadId())
ALL_SI = 0x7
user32.GetUserObjectSecurity.argtypes = [wintypes.HANDLE, ctypes.POINTER(wintypes.DWORD), ctypes.c_void_p, wintypes.DWORD, ctypes.POINTER(wintypes.DWORD)]
user32.GetUserObjectSecurity.restype = wintypes.BOOL
user32.SetUserObjectSecurity.argtypes = [wintypes.HANDLE, ctypes.POINTER(wintypes.DWORD), ctypes.c_void_p]
user32.SetUserObjectSecurity.restype = wintypes.BOOL
advapi32.ConvertSecurityDescriptorToStringSecurityDescriptorW.argtypes = [ctypes.c_void_p, wintypes.DWORD, wintypes.DWORD, ctypes.POINTER(ctypes.c_wchar_p)]
advapi32.ConvertSecurityDescriptorToStringSecurityDescriptorW.restype = wintypes.BOOL
advapi32.ConvertStringSecurityDescriptorToSecurityDescriptorW.argtypes = [wintypes.LPCWSTR, wintypes.DWORD, ctypes.POINTER(ctypes.c_void_p), ctypes.POINTER(ctypes.c_ulong)]
advapi32.ConvertStringSecurityDescriptorToSecurityDescriptorW.restype = wintypes.BOOL
kernel32.LocalFree.argtypes = [ctypes.c_void_p]
kernel32.LocalFree.restype = ctypes.c_void_p

def get_sddl(h):
    siw = wintypes.DWORD(ALL_SI)
    need = wintypes.DWORD()
    small = ctypes.create_string_buffer(1)
    user32.GetUserObjectSecurity(h, ctypes.byref(siw), ctypes.cast(small, ctypes.c_void_p), 0, ctypes.byref(need))
    buf = ctypes.create_string_buffer(max(need.value, 1))
    if not user32.GetUserObjectSecurity(h, ctypes.byref(siw), ctypes.cast(buf, ctypes.c_void_p), ctypes.sizeof(buf), ctypes.byref(need)):
        return None
    sd = ctypes.c_wchar_p()
    if not advapi32.ConvertSecurityDescriptorToStringSecurityDescriptorW(ctypes.cast(buf, ctypes.c_void_p), 1, ALL_SI, ctypes.byref(sd)):
        return None
    r = sd.value
    kernel32.LocalFree(sd)
    return r

def set_sddl(h, sddl):
    sd = ctypes.c_void_p()
    sz = ctypes.c_ulong()
    if not advapi32.ConvertStringSecurityDescriptorToSecurityDescriptorW(sddl, 1, ctypes.byref(sd), ctypes.byref(sz)):
        return False
    siw = wintypes.DWORD(ALL_SI)
    ok = user32.SetUserObjectSecurity(h, ctypes.byref(siw), sd.value)
    kernel32.LocalFree(sd)
    return bool(ok)

def lookup_sid(acct):
    advapi32.LookupAccountNameW.argtypes = [wintypes.LPCWSTR, wintypes.LPCWSTR, ctypes.c_void_p, ctypes.POINTER(wintypes.DWORD), wintypes.LPWSTR, ctypes.POINTER(wintypes.DWORD), ctypes.POINTER(wintypes.DWORD)]
    advapi32.LookupAccountNameW.restype = wintypes.BOOL
    cch = wintypes.DWORD(0)
    dom = ctypes.create_unicode_buffer(256)
    cd = wintypes.DWORD(256)
    use = wintypes.DWORD()
    advapi32.LookupAccountNameW(None, acct, None, ctypes.byref(cch), dom, ctypes.byref(cd), ctypes.byref(use))
    sb = ctypes.create_string_buffer(max(cch.value, 8))
    if not advapi32.LookupAccountNameW(None, acct, ctypes.cast(sb, ctypes.c_void_p), ctypes.byref(cch), dom, ctypes.byref(cd), ctypes.byref(use)):
        return None
    advapi32.ConvertSidToStringSidW.argtypes = [ctypes.c_void_p, ctypes.POINTER(ctypes.c_wchar_p)]
    advapi32.ConvertSidToStringSidW.restype = wintypes.BOOL
    p = ctypes.c_wchar_p()
    if not advapi32.ConvertSidToStringSidW(ctypes.cast(sb, ctypes.c_void_p), ctypes.byref(p)):
        return None
    return p.value

agent_sid = lookup_sid('AgentUser')
ws0 = get_sddl(hwinsta)
ds0 = get_sddl(hdesk)
def add_ace(sddl, sid):
    i = sddl.find('D:(')
    if i < 0:
        return None
    i += 2
    return sddl[:i] + '(A;;GA;;;' + sid + ')' + sddl[i:]

def grant_sids(sids):
    ok1 = set_sddl(hwinsta, add_ace(ws0, sids[0]))
    ok2 = set_sddl(hdesk, add_ace(ds0, sids[0]))
    for s in sids[1:]:
        curw = get_sddl(hwinsta)
        curd = get_sddl(hdesk)
        if s not in curw:
            ok1 = set_sddl(hwinsta, add_ace(curw, s)) and ok1
        if s not in curd:
            ok2 = set_sddl(hdesk, add_ace(curd, s)) and ok2
    return ok1 and ok2

class STARTUPINFOW(ctypes.Structure):
    _fields_ = [('cb', wintypes.DWORD), ('lpReserved', wintypes.LPWSTR), ('lpDesktop', wintypes.LPWSTR), ('lpTitle', wintypes.LPWSTR), ('dwX', wintypes.DWORD), ('dwY', wintypes.DWORD), ('dwXSize', wintypes.DWORD), ('dwYSize', wintypes.DWORD), ('dwXCountChars', wintypes.DWORD), ('dwYCountChars', wintypes.DWORD), ('dwFillAttribute', wintypes.DWORD), ('dwFlags', wintypes.DWORD), ('wShowWindow', wintypes.WORD), ('cbReserved2', wintypes.WORD), ('lpReserved2', ctypes.POINTER(wintypes.BYTE)), ('hStdInput', wintypes.HANDLE), ('hStdOutput', wintypes.HANDLE), ('hStdError', wintypes.HANDLE)]

class STARTUPINFOEX(ctypes.Structure):
    _fields_ = [('StartupInfo', STARTUPINFOW), ('lpAttributeList', ctypes.c_void_p)]

class SEC_CAPS(ctypes.Structure):
    _fields_ = [('AppContainerSid', ctypes.c_void_p), ('Capabilities', ctypes.c_void_p), ('CapabilityCount', wintypes.DWORD), ('Reserved', wintypes.DWORD)]

class PI(ctypes.Structure):
    _fields_ = [('hProcess', wintypes.HANDLE), ('hThread', wintypes.HANDLE), ('dwProcessId', wintypes.DWORD), ('dwThreadId', wintypes.DWORD)]

advapi32.LogonUserW.argtypes = [wintypes.LPCWSTR, wintypes.LPCWSTR, wintypes.LPCWSTR, wintypes.DWORD, wintypes.DWORD, ctypes.POINTER(wintypes.HANDLE)]
advapi32.LogonUserW.restype = wintypes.BOOL
advapi32.CreateProcessAsUserW.argtypes = [wintypes.HANDLE, wintypes.LPCWSTR, wintypes.LPWSTR, ctypes.c_void_p, ctypes.c_void_p, wintypes.BOOL, wintypes.DWORD, ctypes.c_void_p, wintypes.LPCWSTR, ctypes.c_void_p, ctypes.POINTER(PI)]
advapi32.CreateProcessAsUserW.restype = wintypes.BOOL
advapi32.SetTokenInformation.argtypes = [wintypes.HANDLE, ctypes.c_int, ctypes.c_void_p, wintypes.DWORD]
advapi32.SetTokenInformation.restype = wintypes.BOOL
userenv.CreateAppContainerProfile.argtypes = [wintypes.LPCWSTR, wintypes.LPCWSTR, wintypes.LPCWSTR, ctypes.c_void_p, wintypes.DWORD, ctypes.POINTER(ctypes.c_void_p)]
userenv.CreateAppContainerProfile.restype = wintypes.LONG
userenv.DeriveAppContainerSidFromAppContainerName.argtypes = [wintypes.LPCWSTR, ctypes.POINTER(ctypes.c_void_p)]
userenv.DeriveAppContainerSidFromAppContainerName.restype = wintypes.LONG

def set_virt(tok, val):
    v = wintypes.BOOL(val)
    return bool(advapi32.SetTokenInformation(tok, 24, ctypes.byref(v), ctypes.sizeof(v)))

def set_lowil(tok):
    sid = ctypes.c_void_p()
    auth = (ctypes.c_ubyte * 6)(0, 0, 0, 0, 0, 16)
    advapi32.AllocateAndInitializeSid.argtypes = [ctypes.POINTER(ctypes.c_ubyte * 6), wintypes.DWORD] + [wintypes.DWORD] * 8 + [ctypes.POINTER(ctypes.c_void_p)]
    advapi32.AllocateAndInitializeSid.restype = wintypes.BOOL
    if not advapi32.AllocateAndInitializeSid(auth, 1, 4096, 0, 0, 0, 0, 0, 0, 0, ctypes.byref(sid)):
        return False
    class SAA(ctypes.Structure):
        _fields_ = [('Sid', ctypes.c_void_p), ('Attributes', wintypes.DWORD)]
    class TML(ctypes.Structure):
        _fields_ = [('Label', SAA)]
    label = TML()
    label.Label.Sid = sid
    label.Label.Attributes = 0x20
    ok = bool(advapi32.SetTokenInformation(tok, 25, ctypes.byref(label), ctypes.sizeof(label)))
    advapi32.FreeSid.argtypes = [ctypes.c_void_p]
    advapi32.FreeSid(sid)
    return ok

def env_block(env):
    items = []
    for k, v in env.items():
        if not k or '=' in k or '\x00' in k or '\x00' in str(v):
            continue
        items.append(str(k) + '=' + str(v))
    return ('\x00'.join(items) + '\x00\x00')

full_env = dict(os.environ)
full_env.update({'USERNAME': 'AgentUser', 'USERPROFILE': r'C:\Users\AgentUser', 'TEMP': r'C:\workspace\high-nist\.tmp', 'TMP': r'C:\workspace\high-nist\.tmp'})
env_s = env_block(full_env)

APP = 'S4ReproApp5'
profsid = ctypes.c_void_p()
userenv.CreateAppContainerProfile(APP, APP, APP, None, 0, ctypes.byref(profsid))
appsid = ctypes.c_void_p()
userenv.DeriveAppContainerSidFromAppContainerName(APP, ctypes.byref(appsid))

def appc_sid_str():
    advapi32.ConvertSidToStringSidW.argtypes = [ctypes.c_void_p, ctypes.POINTER(ctypes.c_wchar_p)]
    advapi32.ConvertSidToStringSidW.restype = wintypes.BOOL
    p = ctypes.c_wchar_p()
    if not advapi32.ConvertSidToStringSidW(appsid, ctypes.byref(p)):
        return None
    r = p.value
    kernel32.LocalFree(p)
    return r

appc_str = appc_sid_str()
log('APPC_SID', appc_str)
log('GRANT', grant_sids([agent_sid, appc_str]))

kernel32.CreatePipe.argtypes = [ctypes.POINTER(wintypes.HANDLE), ctypes.POINTER(wintypes.HANDLE), ctypes.c_void_p, wintypes.DWORD]
kernel32.CreatePipe.restype = wintypes.BOOL
kernel32.SetHandleInformation.argtypes = [wintypes.HANDLE, wintypes.DWORD, wintypes.DWORD]
kernel32.SetHandleInformation.restype = wintypes.BOOL
kernel32.CreateFileW.argtypes = [wintypes.LPCWSTR, wintypes.DWORD, wintypes.DWORD, ctypes.c_void_p, wintypes.DWORD, wintypes.DWORD, wintypes.HANDLE]
kernel32.CreateFileW.restype = wintypes.HANDLE

def make_pipes():
    rh, wh = wintypes.HANDLE(), wintypes.HANDLE()
    if not kernel32.CreatePipe(ctypes.byref(rh), ctypes.byref(wh), None, 0):
        return None, None
    kernel32.SetHandleInformation(wh, 0x1, 0x1)
    return rh, wh

nul = wintypes.HANDLE(kernel32.CreateFileW('NUL', 0x80000000, 0x3, None, 3, 0, None))

def wait_exit(pi, label):
    kernel32.WaitForSingleObject.argtypes = [wintypes.HANDLE, wintypes.DWORD]
    kernel32.WaitForSingleObject.restype = wintypes.DWORD
    kernel32.WaitForSingleObject(pi.hProcess, 20000)
    code = wintypes.DWORD()
    kernel32.GetExitCodeProcess.argtypes = [wintypes.HANDLE, ctypes.POINTER(wintypes.DWORD)]
    kernel32.GetExitCodeProcess.restype = wintypes.BOOL
    kernel32.GetExitCodeProcess(pi.hProcess, ctypes.byref(code))
    log(label, 'EXIT', hex(code.value) if code.value > 0x7fffffff else code.value)
    kernel32.CloseHandle(pi.hProcess)
    kernel32.CloseHandle(pi.hThread)

def spawn(label, use_appc=True, use_suspend=False, use_pipes=False, use_limits=False, cmdline='cmd.exe /c exit 42'):
    tok = wintypes.HANDLE()
    advapi32.LogonUserW('AgentUser', 'DESKTOP-QMAPMFH', 'S4Run!2026', 4, 0, ctypes.byref(tok))
    set_virt(tok, False)
    set_lowil(tok)
    flags = NO_WINDOW | NEW_GROUP | UNICODE_ENV | EXTENDED
    if use_suspend:
        flags |= SUSPENDED
    eb = ctypes.create_unicode_buffer(env_s)
    env_ptr = ctypes.cast(eb, ctypes.c_void_p)
    nattr = 2 if use_appc else 1
    kernel32.InitializeProcThreadAttributeList.argtypes = [ctypes.c_void_p, wintypes.DWORD, wintypes.DWORD, ctypes.POINTER(ctypes.c_size_t)]
    kernel32.InitializeProcThreadAttributeList.restype = wintypes.BOOL
    sz = ctypes.c_size_t()
    kernel32.InitializeProcThreadAttributeList(None, nattr, 0, ctypes.byref(sz))
    attr_list = ctypes.create_string_buffer(sz.value)
    kernel32.InitializeProcThreadAttributeList(ctypes.cast(attr_list, ctypes.c_void_p), nattr, 0, ctypes.byref(sz))
    kernel32.UpdateProcThreadAttribute.argtypes = [ctypes.c_void_p, wintypes.DWORD, ctypes.c_size_t, ctypes.c_void_p, ctypes.c_size_t, ctypes.c_void_p, ctypes.c_void_p]
    kernel32.UpdateProcThreadAttribute.restype = wintypes.BOOL
    kernel32.CreateJobObjectW.argtypes = [ctypes.c_void_p, wintypes.LPCWSTR]
    kernel32.CreateJobObjectW.restype = wintypes.HANDLE
    job = wintypes.HANDLE(kernel32.CreateJobObjectW(None, None))
    if use_limits:
        class JOBOBJECT_EXTENDED_LIMIT_INFORMATION(ctypes.Structure):
            _fields_ = [('BasicLimitInformation', ctypes.c_ubyte * 64), ('IoInfo', ctypes.c_ubyte * 48), ('ProcessMemoryLimit', ctypes.c_size_t), ('JobMemoryLimit', ctypes.c_size_t), ('PeakProcessMemoryUsed', ctypes.c_size_t), ('PeakJobMemoryUsed', ctypes.c_size_t)]
        kernel32.SetInformationJobObject.argtypes = [wintypes.HANDLE, ctypes.c_int, ctypes.c_void_p, wintypes.DWORD]
        kernel32.SetInformationJobObject.restype = wintypes.BOOL
        info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION()
        info.JobMemoryLimit = 512 * 1024 * 1024
        info.BasicLimitInformation = (ctypes.c_ubyte * 64)(*(0,) * 64)
        kernel32.SetInformationJobObject(job, 9, ctypes.byref(info), ctypes.sizeof(info))
    kernel32.UpdateProcThreadAttribute(ctypes.cast(attr_list, ctypes.c_void_p), 0, 0x0002000B, ctypes.byref(job), ctypes.sizeof(wintypes.HANDLE), None, None)
    if use_appc:
        sc = SEC_CAPS()
        sc.AppContainerSid = appsid
        sc.Capabilities = None
        sc.CapabilityCount = 0
        sc.Reserved = 0
        kernel32.UpdateProcThreadAttribute(ctypes.cast(attr_list, ctypes.c_void_p), 0, 0x20009, ctypes.byref(sc), ctypes.sizeof(sc), None, None)
    si = STARTUPINFOEX()
    si.StartupInfo.cb = ctypes.sizeof(STARTUPINFOEX)
    si.lpAttributeList = ctypes.cast(attr_list, ctypes.c_void_p)
    if use_pipes:
        r1, w1 = make_pipes()
        r2, w2 = make_pipes()
        si.StartupInfo.dwFlags |= 0x100
        si.StartupInfo.hStdOutput = w1
        si.StartupInfo.hStdError = w2 or w1
        si.StartupInfo.hStdInput = nul
    cmd = ctypes.create_unicode_buffer(cmdline)
    pi = PI()
    ok = advapi32.CreateProcessAsUserW(tok, None, cmd, None, None, True, flags, env_ptr, 'C:\\workspace\\high-nist', ctypes.cast(ctypes.byref(si), ctypes.c_void_p), ctypes.byref(pi))
    log(label, 'CPAU', ok, 'err', ctypes.get_last_error())
    if not ok:
        return
    if use_suspend:
        kernel32.ResumeThread.argtypes = [wintypes.HANDLE]
        kernel32.ResumeThread.restype = wintypes.DWORD
        kernel32.ResumeThread(pi.hThread)
    wait_exit(pi, label)
    kernel32.DeleteProcThreadAttributeList.argtypes = [ctypes.c_void_p]
    kernel32.DeleteProcThreadAttributeList(ctypes.cast(attr_list, ctypes.c_void_p))
    kernel32.CloseHandle(tok)

spawn('A_NO_SUSP_NO_PIPES', True, False, False)
spawn('B_NO_SUSP_PIPES', True, False, True)
spawn('C_SUSP_NO_PIPES', True, True, False)
spawn('D_SUSP_PIPES', True, True, True)
spawn('E_SUSP_PIPES_LIMITS', True, True, True, use_limits=True)
spawn('F_POWERSHELL_REPLICA', True, True, True, use_limits=True, cmdline='powershell.exe -NoProfile -NonInteractive -Command exit 42')
spawn('G_POWERSHELL_NO_APPC', False, True, True, use_limits=True, cmdline='powershell.exe -NoProfile -NonInteractive -Command exit 42')

set_sddl(hwinsta, ws0)
set_sddl(hdesk, ds0)
log('RESTORED')
log('DONE')
"@
Set-Content -LiteralPath 'C:\s4\tools\repro-v10.py' -Value $pyCode -Encoding utf8
& $py 'C:\s4\tools\repro-v10.py' 2>&1 | ForEach-Object { Write-Output "PY: $_" }
'@

$bodyFile = 'D:\CLI\_windows_high_nist\job-repro-v10-body.ps1'
Set-Content -LiteralPath $bodyFile -Value $body -Encoding utf8
try {
    $sec = ConvertTo-SecureString '123456' -AsPlainText -Force
    $cred = New-Object System.Management.Automation.PSCredential('HL', $sec)
    $sess = New-PSSession -VMName 'win-s4' -Credential $cred
    $bodyBytes = [System.Text.Encoding]::UTF8.GetBytes($body)
    $b64 = [Convert]::ToBase64String($bodyBytes)
    Invoke-Command -Session $sess -ArgumentList $b64 -ScriptBlock {
        param($B64)
        New-Item -ItemType Directory -Path 'C:\s4\tools' -Force | Out-Null
        $scriptText = [System.Text.Encoding]::UTF8.GetString([Convert]::FromBase64String($B64))
        Set-Content -LiteralPath 'C:\s4\tools\v10-body.ps1' -Value $scriptText -Encoding utf8
        $tr = '"powershell.exe -NoProfile -ExecutionPolicy Bypass -File C:\s4\tools\v10-runner.ps1"'
        $runnerCode = @'
$log = 'C:\s4\tools\v10-job.log'
Remove-Item -LiteralPath $log -Force -ErrorAction SilentlyContinue
try {
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File 'C:\s4\tools\v10-body.ps1' *> $log
    $code = $LASTEXITCODE
    if ($null -eq $code) { $code = 0 }
} catch {
    "ERROR=$($_.Exception.Message)" | Out-File -Encoding utf8 $log
    $code = 1
}
"EXIT=$code" | Add-Content -LiteralPath $log -Encoding utf8
'@
        Set-Content -LiteralPath 'C:\s4\tools\v10-runner.ps1' -Value $runnerCode -Encoding utf8
        Remove-Item -LiteralPath 'C:\s4\tools\v10-job.log' -Force -ErrorAction SilentlyContinue
        & schtasks.exe /create /f /tn s4elevv10 /tr $tr /sc once /st 23:59 /ru SYSTEM /rl highest 2>&1 | Out-Null
        if ($LASTEXITCODE -ne 0) { throw 'schtasks create s4elevv10 failed' }
        $out = & schtasks.exe /run /tn s4elevv10 2>&1
        if ($LASTEXITCODE -ne 0) { throw "schtasks run s4elevv10 failed: $($out -join ' ')" }
        return $true
    }
    $deadline = (Get-Date).AddSeconds(400)
    $done = $false
    while ((Get-Date) -lt $deadline) {
        Start-Sleep -Seconds 2
        $last = Invoke-Command -Session $sess -ScriptBlock {
            if (Test-Path -LiteralPath 'C:\s4\tools\v10-job.log') {
                Get-Content -LiteralPath 'C:\s4\tools\v10-job.log' -Tail 1
            }
        }
        if ($last -match '^EXIT=') { $done = $true; break }
    }
    if ($done) {
        $logContent = Invoke-Command -Session $sess -ScriptBlock {
            Get-Content -LiteralPath 'C:\s4\tools\v10-job.log'
        }
        foreach ($l in $logContent) { $lines.Add($l) }
    } else {
        $lines.Add('JOB_TIMEOUT')
    }
    Remove-PSSession -Session $sess -ErrorAction SilentlyContinue
}
catch {
    $lines.Add("ERROR: $($_.Exception.Message)")
    $lines.Add("STACK: $($_.ScriptStackTrace)")
}
$lines | Set-Content -LiteralPath $OutFile -Encoding utf8
