<#
    S4 VM repro v3 (host-side elevated):
      1. Env-block matrix on CreateProcessW (admin token): unicode buffer,
         c_char_p kept alive, c_char_p temp-cast, no-flag control, full env.
      2. Capture subprocess's actual env argument (via _winapi wrapper).
      3. Window-station/desktop hypothesis for 0xC0000142:
         - report session id, winsta/desktop names + SDDL
         - spawn cmd.exe as AgentUser before grant (expect 0xC0000142)
         - grant AgentUser SID on winsta+desktop, respawn (env + explicit desktop)
         - CreateProcessWithLogonW comparison
         - restore SDDL and respawn to confirm the ACL is the factor
    ASCII only.
#>
[CmdletBinding()]
param(
    [string]$OutFile = 'D:\CLI\_windows_high_nist\vm-repro-v3-result.txt'
)

$ErrorActionPreference = 'Stop'
$lines = New-Object System.Collections.Generic.List[string]
$runner = 'D:\CLI\scripts\s4_vm_run_elev.ps1'

# Wait for the guest to be reachable.
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
New-Item -ItemType Directory -Path 'C:\s4\diag-ws' -Force | Out-Null
New-Item -ItemType Directory -Path 'C:\workspace' -Force | Out-Null
$pyCode = @"
import ctypes, os, subprocess, sys
from ctypes import wintypes

def log(*a):
    print(' '.join(str(x) for x in a), flush=True)

kernel32 = ctypes.WinDLL('kernel32', use_last_error=True)
advapi32 = ctypes.WinDLL('advapi32', use_last_error=True)
user32 = ctypes.WinDLL('user32', use_last_error=True)

# --- session ---
kernel32.ProcessIdToSessionId.argtypes = [wintypes.DWORD, ctypes.POINTER(wintypes.DWORD)]
kernel32.ProcessIdToSessionId.restype = wintypes.BOOL
sess = wintypes.DWORD()
kernel32.ProcessIdToSessionId(os.getpid(), ctypes.byref(sess))
log('SESSION', sess.value)

# --- winsta / desktop handles ---
user32.GetProcessWindowStation.argtypes = []
user32.GetProcessWindowStation.restype = wintypes.HANDLE
hwinsta = user32.GetProcessWindowStation()
kernel32.GetCurrentThreadId.argtypes = []
kernel32.GetCurrentThreadId.restype = wintypes.DWORD
user32.GetThreadDesktop.argtypes = [wintypes.DWORD]
user32.GetThreadDesktop.restype = wintypes.HANDLE
hdesk = user32.GetThreadDesktop(kernel32.GetCurrentThreadId())
user32.GetUserObjectInformationW.argtypes = [wintypes.HANDLE, ctypes.c_int, ctypes.c_void_p, wintypes.DWORD, ctypes.POINTER(wintypes.DWORD)]
user32.GetUserObjectInformationW.restype = wintypes.BOOL

def obj_name(h):
    buf = ctypes.create_unicode_buffer(512)
    need = wintypes.DWORD()
    if user32.GetUserObjectInformationW(h, 2, ctypes.cast(buf, ctypes.c_void_p), ctypes.sizeof(buf), ctypes.byref(need)):
        return buf.value
    return '?'

wname = obj_name(hwinsta)
dname = obj_name(hdesk)
log('WINSTA', wname)
log('DESKTOP', dname)

# --- SDDL get/set ---
DACL_SI = 0x4
user32.GetUserObjectSecurity.argtypes = [wintypes.HANDLE, ctypes.POINTER(wintypes.DWORD), ctypes.c_void_p, wintypes.DWORD, ctypes.POINTER(wintypes.DWORD)]
user32.GetUserObjectSecurity.restype = wintypes.BOOL

def get_sddl(h):
    si = wintypes.DWORD(DACL_SI)
    need = wintypes.DWORD()
    if not user32.GetUserObjectSecurity(h, ctypes.byref(si), None, 0, ctypes.byref(need)):
        return None
    buf = ctypes.create_string_buffer(need.value)
    if not user32.GetUserObjectSecurity(h, ctypes.byref(si), ctypes.cast(buf, ctypes.c_void_p), need.value, ctypes.byref(need)):
        return None
    advapi32.ConvertSecurityDescriptorToStringSecurityDescriptorW.argtypes = [ctypes.c_void_p, wintypes.DWORD, wintypes.DWORD, ctypes.POINTER(ctypes.c_wchar_p)]
    advapi32.ConvertSecurityDescriptorToStringSecurityDescriptorW.restype = wintypes.BOOL
    sd = ctypes.c_wchar_p()
    if not advapi32.ConvertSecurityDescriptorToStringSecurityDescriptorW(ctypes.cast(buf, ctypes.c_void_p), 1, DACL_SI, ctypes.byref(sd)):
        return None
    return sd.value

def set_sddl(h, sddl):
    advapi32.ConvertStringSecurityDescriptorToSecurityDescriptorW.argtypes = [wintypes.LPCWSTR, wintypes.DWORD, ctypes.POINTER(ctypes.c_void_p), ctypes.POINTER(ctypes.c_ulong)]
    advapi32.ConvertStringSecurityDescriptorToSecurityDescriptorW.restype = wintypes.BOOL
    sd = ctypes.c_void_p()
    sz = ctypes.c_ulong()
    if not advapi32.ConvertStringSecurityDescriptorToSecurityDescriptorW(sddl, 1, ctypes.byref(sd), ctypes.byref(sz)):
        return False, 'conv ' + str(ctypes.get_last_error())
    user32.SetUserObjectSecurity.argtypes = [wintypes.HANDLE, ctypes.POINTER(wintypes.DWORD), ctypes.c_void_p]
    user32.SetUserObjectSecurity.restype = wintypes.BOOL
    si = wintypes.DWORD(DACL_SI)
    ok = user32.SetUserObjectSecurity(h, ctypes.byref(si), sd.value)
    err = ctypes.get_last_error()
    advapi32.LocalFree.argtypes = [ctypes.c_void_p]
    advapi32.LocalFree(sd)
    return bool(ok), 'set ' + str(err)

ws_sddl0 = get_sddl(hwinsta)
ds_sddl0 = get_sddl(hdesk)
log('WINSTA_SDDL', ws_sddl0)
log('DESKTOP_SDDL', ds_sddl0)

# --- AgentUser SID ---
advapi32.LookupAccountNameW.argtypes = [wintypes.LPCWSTR, wintypes.LPCWSTR, ctypes.c_void_p, ctypes.POINTER(wintypes.DWORD), wintypes.LPWSTR, ctypes.POINTER(wintypes.DWORD), ctypes.POINTER(wintypes.DWORD)]
advapi32.LookupAccountNameW.restype = wintypes.BOOL

def lookup_sid(acct):
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
log('AGENT_SID', agent_sid)

def add_ace(sddl, sid):
    if sddl is None:
        return None
    i = sddl.find('D:(')
    if i < 0:
        return None
    i += 3
    return sddl[:i] + '(A;;GA;;;' + sid + ')' + sddl[i:]

new_ws = add_ace(ws_sddl0, agent_sid)
new_ds = add_ace(ds_sddl0, agent_sid)
log('NEW_WINSTA', new_ws)
log('NEW_DESKTOP', new_ds)

# --- structures ---
class STARTUPINFOW(ctypes.Structure):
    _fields_ = [('cb', wintypes.DWORD), ('lpReserved', wintypes.LPWSTR), ('lpDesktop', wintypes.LPWSTR), ('lpTitle', wintypes.LPWSTR), ('dwX', wintypes.DWORD), ('dwY', wintypes.DWORD), ('dwXSize', wintypes.DWORD), ('dwYSize', wintypes.DWORD), ('dwXCountChars', wintypes.DWORD), ('dwYCountChars', wintypes.DWORD), ('dwFillAttribute', wintypes.DWORD), ('dwFlags', wintypes.DWORD), ('wShowWindow', wintypes.WORD), ('cbReserved2', wintypes.WORD), ('lpReserved2', ctypes.c_void_p), ('hStdInput', wintypes.HANDLE), ('hStdOutput', wintypes.HANDLE), ('hStdError', wintypes.HANDLE)]

class PI(ctypes.Structure):
    _fields_ = [('hProcess', wintypes.HANDLE), ('hThread', wintypes.HANDLE), ('dwProcessId', wintypes.DWORD), ('dwThreadId', wintypes.DWORD)]

def wait_exit(pi, label):
    kernel32.WaitForSingleObject.argtypes = [wintypes.HANDLE, wintypes.DWORD]
    kernel32.WaitForSingleObject.restype = wintypes.DWORD
    kernel32.WaitForSingleObject(pi.hProcess, 15000)
    code = wintypes.DWORD()
    kernel32.GetExitCodeProcess.argtypes = [wintypes.HANDLE, ctypes.POINTER(wintypes.DWORD)]
    kernel32.GetExitCodeProcess.restype = wintypes.BOOL
    kernel32.GetExitCodeProcess(pi.hProcess, ctypes.byref(code))
    if code.value > 0x7fffffff:
        log(label, 'EXIT', hex(code.value))
    else:
        log(label, 'EXIT', code.value)
    kernel32.CloseHandle(pi.hProcess)
    kernel32.CloseHandle(pi.hThread)

# --- env builders ---
def env_str(env):
    items = []
    for k, v in env.items():
        if not k or '=' in k or '\x00' in k:
            continue
        if '\x00' in str(v):
            continue
        items.append(str(k) + '=' + str(v))
    return '\x00'.join(items) + '\x00\x00'

min_env = {'PATH': os.environ.get('PATH', '')}
min_str = env_str(min_env)
full_str = env_str(os.environ)

# --- CreateProcessW env matrix (admin token) ---
kernel32.CreateProcessW.argtypes = [wintypes.LPCWSTR, wintypes.LPWSTR, ctypes.c_void_p, ctypes.c_void_p, wintypes.BOOL, wintypes.DWORD, ctypes.c_void_p, wintypes.LPCWSTR, ctypes.c_void_p, ctypes.POINTER(PI)]
kernel32.CreateProcessW.restype = wintypes.BOOL

def spawn_cw(label, mode, flags=0x00000400):
    cmd = ctypes.create_unicode_buffer('cmd.exe /c exit 42')
    env_ptr = None
    hold = None
    if mode == 'unibuf':
        buf = ctypes.create_unicode_buffer(min_str)
        hold = buf
        env_ptr = ctypes.cast(buf, ctypes.c_void_p)
        log(label, 'UNIBUF_SZ', ctypes.sizeof(buf), 'BYTES', ctypes.string_at(buf, min(48, ctypes.sizeof(buf))))
    elif mode == 'cp_alive':
        cp = ctypes.c_char_p(min_str.encode('utf-16-le'))
        hold = cp
        env_ptr = ctypes.cast(cp, ctypes.c_void_p)
    elif mode == 'cp_cast':
        env_ptr = ctypes.cast(ctypes.c_char_p(min_str.encode('utf-16-le')), ctypes.c_void_p)
    elif mode == 'full_unibuf':
        buf = ctypes.create_unicode_buffer(full_str)
        hold = buf
        env_ptr = ctypes.cast(buf, ctypes.c_void_p)
    si = STARTUPINFOW()
    si.cb = ctypes.sizeof(si)
    pi = PI()
    ok = kernel32.CreateProcessW(None, cmd, None, None, True, flags, env_ptr, 'C:\\s4\\diag-ws', ctypes.byref(si), ctypes.byref(pi))
    log(label, 'CW', ok, 'err', ctypes.get_last_error())
    if ok:
        wait_exit(pi, label)

spawn_cw('CW_UNIBUF', 'unibuf')
spawn_cw('CW_CP_ALIVE', 'cp_alive')
spawn_cw('CW_CP_CAST', 'cp_cast')
spawn_cw('CW_UNIBUF_NOFLAG', 'unibuf', flags=0)
spawn_cw('CW_FULL_UNIBUF', 'full_unibuf')

# --- subprocess env capture ---
orig_sp = subprocess._winapi.CreateProcess
def spy(*a, **k):
    env_arg = a[6] if len(a) > 6 else None
    if env_arg is not None:
        log('SP_ENV', type(env_arg).__name__, 'LEN', len(env_arg), 'PREFIX', repr(env_arg[:36]))
    else:
        log('SP_ENV', 'None')
    return orig_sp(*a, **k)
subprocess._winapi.CreateProcess = spy
r1 = subprocess.run(['cmd.exe', '/c', 'exit', '42'], env=min_env, capture_output=True)
log('SP_MINENV_RC', r1.returncode)
r2 = subprocess.run(['cmd.exe', '/c', 'exit', '42'], env=os.environ, capture_output=True)
log('SP_FULLENV_RC', r2.returncode)

# --- LogonUser + token spawn ---
advapi32.LogonUserW.argtypes = [wintypes.LPCWSTR, wintypes.LPCWSTR, wintypes.LPCWSTR, wintypes.DWORD, wintypes.DWORD, ctypes.POINTER(wintypes.HANDLE)]
advapi32.LogonUserW.restype = wintypes.BOOL
advapi32.CreateProcessWithTokenW.argtypes = [wintypes.HANDLE, wintypes.DWORD, wintypes.LPCWSTR, wintypes.LPWSTR, wintypes.DWORD, ctypes.c_void_p, wintypes.LPCWSTR, ctypes.c_void_p, ctypes.POINTER(PI)]
advapi32.CreateProcessWithTokenW.restype = wintypes.BOOL

def spawn_cptw(label, tok, env_uni=None, desktop=None, cwd='C:\\workspace', flags=0x00000400):
    cmd = ctypes.create_unicode_buffer('cmd.exe /c exit 42')
    si = STARTUPINFOW()
    si.cb = ctypes.sizeof(si)
    hold = []
    if desktop:
        db = ctypes.create_unicode_buffer(desktop)
        hold.append(db)
        si.lpDesktop = ctypes.cast(db, wintypes.LPWSTR)
    env_ptr = None
    if env_uni is not None:
        eb = ctypes.create_unicode_buffer(env_uni)
        hold.append(eb)
        env_ptr = ctypes.cast(eb, ctypes.c_void_p)
    pi = PI()
    ok = advapi32.CreateProcessWithTokenW(tok, 0, None, cmd, flags, env_ptr, cwd, ctypes.byref(si), ctypes.byref(pi))
    log(label, 'CREATE', ok, 'err', ctypes.get_last_error())
    if ok:
        wait_exit(pi, label)

lt = wintypes.HANDLE()
ok_l = advapi32.LogonUserW('AgentUser', 'DESKTOP-QMAPMFH', 'S4Run!2026', 4, 0, ctypes.byref(lt))
log('LOGON', ok_l, ctypes.get_last_error())

if ok_l:
    spawn_cptw('CPTW_NOENV_BEFORE', lt)
    g1 = set_sddl(hwinsta, new_ws)
    g2 = set_sddl(hdesk, new_ds)
    log('GRANT_WINSTA', g1)
    log('GRANT_DESKTOP', g2)
    spawn_cptw('CPTW_NOENV_AFTER', lt)
    spawn_cptw('CPTW_UNIBUF_ENV', lt, env_uni=min_str)
    spawn_cptw('CPTW_EXPLICIT_DESKTOP', lt, desktop=wname + '\\\\' + dname)
    spawn_cptw('CPTW_FULL_ENV', lt, env_uni=full_str)

# --- CreateProcessWithLogonW comparison (no env) ---
advapi32.CreateProcessWithLogonW.argtypes = [wintypes.LPCWSTR, wintypes.LPWSTR, wintypes.DWORD, wintypes.LPCWSTR, wintypes.LPCWSTR, wintypes.LPCWSTR, wintypes.DWORD, ctypes.c_void_p, wintypes.LPCWSTR, ctypes.c_void_p, ctypes.POINTER(PI)]
advapi32.CreateProcessWithLogonW.restype = wintypes.BOOL

def spawn_logonw(label, flags=0, desktop=None):
    cmd = ctypes.create_unicode_buffer('cmd.exe /c exit 42')
    si = STARTUPINFOW()
    si.cb = ctypes.sizeof(si)
    hold = []
    if desktop:
        db = ctypes.create_unicode_buffer(desktop)
        hold.append(db)
        si.lpDesktop = ctypes.cast(db, wintypes.LPWSTR)
    pi = PI()
    ok = advapi32.CreateProcessWithLogonW(None, cmd, 0, 'AgentUser', 'DESKTOP-QMAPMFH', 'S4Run!2026', flags, None, 'C:\\workspace', ctypes.byref(si), ctypes.byref(pi))
    log(label, 'LOGONW', ok, 'err', ctypes.get_last_error())
    if ok:
        wait_exit(pi, label)

spawn_logonw('LOGONW_NOENV')

# --- restore and confirm ---
if ok_l:
    r1s = set_sddl(hwinsta, ws_sddl0)
    r2s = set_sddl(hdesk, ds_sddl0)
    log('RESTORE_WINSTA', r1s)
    log('RESTORE_DESKTOP', r2s)
    spawn_cptw('CPTW_NOENV_RESTORED', lt)
log('DONE')
"@
Set-Content -LiteralPath 'C:\s4\tools\repro-v3.py' -Value $pyCode -Encoding utf8
& $py 'C:\s4\tools\repro-v3.py' 2>&1 | ForEach-Object { Write-Output "PY: $_" }
'@

$bodyFile = 'D:\CLI\_windows_high_nist\job-repro-v3-body.ps1'
Set-Content -LiteralPath $bodyFile -Value $body -Encoding utf8
try {
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $runner `
        -JobBodyFile $bodyFile -OutFile $OutFile -TimeoutSeconds 400
    if ($LASTEXITCODE -ne 0) { throw "repro-v3 job failed (exit $LASTEXITCODE)" }
}
catch {
    $lines.Add("ERROR: $($_.Exception.Message)")
    $lines | Set-Content -LiteralPath $OutFile -Encoding utf8
}
