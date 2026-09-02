<#
    S4 VM repro v4 (host-side elevated):
      - Env-block matrix on CreateProcessW AND CreateProcessWithTokenW:
        unicode buffer + flag, c_char_p + flag, ANSI block without flag.
      - Capture subprocess env argument type (dict/bytes).
      - Window-station/desktop ACL grant test with guaranteed restore
        (try/finally), full O/G/D SDDL read/write with error logging.
    ASCII only.
#>
[CmdletBinding()]
param(
    [string]$OutFile = 'D:\CLI\_windows_high_nist\vm-repro-v4-result.txt'
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

kernel32.ProcessIdToSessionId.argtypes = [wintypes.DWORD, ctypes.POINTER(wintypes.DWORD)]
kernel32.ProcessIdToSessionId.restype = wintypes.BOOL
sess = wintypes.DWORD()
kernel32.ProcessIdToSessionId(os.getpid(), ctypes.byref(sess))
log('SESSION', sess.value)

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

# --- SDDL get/set with full O/G/D information ---
ALL_SI = 0x1 | 0x2 | 0x4
DACL_SI = 0x4
user32.GetUserObjectSecurity.argtypes = [wintypes.HANDLE, ctypes.POINTER(wintypes.DWORD), ctypes.c_void_p, wintypes.DWORD, ctypes.POINTER(wintypes.DWORD)]
user32.GetUserObjectSecurity.restype = wintypes.BOOL
user32.SetUserObjectSecurity.argtypes = [wintypes.HANDLE, ctypes.POINTER(wintypes.DWORD), ctypes.c_void_p]
user32.SetUserObjectSecurity.restype = wintypes.BOOL

def get_sddl(h, si):
    siw = wintypes.DWORD(si)
    need = wintypes.DWORD()
    small = ctypes.create_string_buffer(1)
    ok1 = user32.GetUserObjectSecurity(h, ctypes.byref(siw), ctypes.cast(small, ctypes.c_void_p), 0, ctypes.byref(need))
    e1 = ctypes.get_last_error()
    if not ok1 and e1 not in (122, 0):
        log('SDDL_SIZE_ERR', hex(h), 'err', e1, 'need', need.value)
    buf = ctypes.create_string_buffer(max(need.value, 1))
    ok2 = user32.GetUserObjectSecurity(h, ctypes.byref(siw), ctypes.cast(buf, ctypes.c_void_p), ctypes.sizeof(buf), ctypes.byref(need))
    if not ok2:
        log('SDDL_READ_ERR', hex(h), 'err', ctypes.get_last_error())
        return None
    advapi32.ConvertSecurityDescriptorToStringSecurityDescriptorW.argtypes = [ctypes.c_void_p, wintypes.DWORD, wintypes.DWORD, ctypes.POINTER(ctypes.c_wchar_p)]
    advapi32.ConvertSecurityDescriptorToStringSecurityDescriptorW.restype = wintypes.BOOL
    sd = ctypes.c_wchar_p()
    if not advapi32.ConvertSecurityDescriptorToStringSecurityDescriptorW(ctypes.cast(buf, ctypes.c_void_p), 1, si, ctypes.byref(sd)):
        log('SDDL_CONVERT_ERR', hex(h), 'err', ctypes.get_last_error())
        return None
    return sd.value

def set_sddl(h, sddl, si):
    advapi32.ConvertStringSecurityDescriptorToSecurityDescriptorW.argtypes = [wintypes.LPCWSTR, wintypes.DWORD, ctypes.POINTER(ctypes.c_void_p), ctypes.POINTER(ctypes.c_ulong)]
    advapi32.ConvertStringSecurityDescriptorToSecurityDescriptorW.restype = wintypes.BOOL
    sd = ctypes.c_void_p()
    sz = ctypes.c_ulong()
    if not advapi32.ConvertStringSecurityDescriptorToSecurityDescriptorW(sddl, 1, ctypes.byref(sd), ctypes.byref(sz)):
        return False, 'conv ' + str(ctypes.get_last_error())
    siw = wintypes.DWORD(si)
    ok = user32.SetUserObjectSecurity(h, ctypes.byref(siw), sd.value)
    err = ctypes.get_last_error()
    kernel32.LocalFree.argtypes = [ctypes.c_void_p]
    kernel32.LocalFree(sd)
    return bool(ok), 'set ' + str(err)

ws_sddl0 = get_sddl(hwinsta, ALL_SI)
ds_sddl0 = get_sddl(hdesk, ALL_SI)
log('WINSTA_SDDL', ws_sddl0)
log('DESKTOP_SDDL', ds_sddl0)

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
    i += 2
    return sddl[:i] + '(A;;GA;;;' + sid + ')' + sddl[i:]

new_ws = add_ace(ws_sddl0, agent_sid)
new_ds = add_ace(ds_sddl0, agent_sid)
log('NEW_WINSTA', new_ws)
log('NEW_DESKTOP', new_ds)

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

# --- subprocess env argument capture (may be a dict in 3.12) ---
orig_sp = subprocess._winapi.CreateProcess
def spy(*a, **k):
    env_arg = a[6] if len(a) > 6 else None
    if env_arg is not None:
        log('SP_ENV', type(env_arg).__name__, 'LEN', len(env_arg))
    else:
        log('SP_ENV', 'None')
    return orig_sp(*a, **k)
subprocess._winapi.CreateProcess = spy
r1 = subprocess.run(['cmd.exe', '/c', 'exit', '42'], env=min_env, capture_output=True)
log('SP_MINENV_RC', r1.returncode)
r2 = subprocess.run(['cmd.exe', '/c', 'exit', '42'], env=os.environ, capture_output=True)
log('SP_FULLENV_RC', r2.returncode)

# --- CreateProcessW env matrix ---
kernel32.CreateProcessW.argtypes = [wintypes.LPCWSTR, wintypes.LPWSTR, ctypes.c_void_p, ctypes.c_void_p, wintypes.BOOL, wintypes.DWORD, ctypes.c_void_p, wintypes.LPCWSTR, ctypes.c_void_p, ctypes.POINTER(PI)]
kernel32.CreateProcessW.restype = wintypes.BOOL

def spawn_cw(label, mode, flags):
    cmd = ctypes.create_unicode_buffer('cmd.exe /c exit 42')
    env_ptr = None
    hold = []
    if mode == 'unibuf':
        buf = ctypes.create_unicode_buffer(min_str)
        hold.append(buf)
        env_ptr = ctypes.cast(buf, ctypes.c_void_p)
    elif mode == 'cp':
        cp = ctypes.c_char_p(min_str.encode('utf-16-le'))
        hold.append(cp)
        env_ptr = ctypes.cast(cp, ctypes.c_void_p)
    elif mode == 'ansi':
        ab = ctypes.create_string_buffer(min_str.encode('mbcs'))
        hold.append(ab)
        env_ptr = ctypes.cast(ab, ctypes.c_void_p)
    si = STARTUPINFOW()
    si.cb = ctypes.sizeof(si)
    pi = PI()
    ok = kernel32.CreateProcessW(None, cmd, None, None, True, flags, env_ptr, 'C:\\s4\\diag-ws', ctypes.byref(si), ctypes.byref(pi))
    log(label, 'CW', ok, 'err', ctypes.get_last_error())
    if ok:
        wait_exit(pi, label)

spawn_cw('CW_UNIBUF', 'unibuf', 0x400)
spawn_cw('CW_CP', 'cp', 0x400)
spawn_cw('CW_ANSI_NOFLAG', 'ansi', 0)

# --- CreateProcessWithTokenW env matrix + spawn ---
advapi32.LogonUserW.argtypes = [wintypes.LPCWSTR, wintypes.LPCWSTR, wintypes.LPCWSTR, wintypes.DWORD, wintypes.DWORD, ctypes.POINTER(wintypes.HANDLE)]
advapi32.LogonUserW.restype = wintypes.BOOL
advapi32.CreateProcessWithTokenW.argtypes = [wintypes.HANDLE, wintypes.DWORD, wintypes.LPCWSTR, wintypes.LPWSTR, wintypes.DWORD, ctypes.c_void_p, wintypes.LPCWSTR, ctypes.c_void_p, ctypes.POINTER(PI)]
advapi32.CreateProcessWithTokenW.restype = wintypes.BOOL

def spawn_cptw(label, tok, mode, flags, cwd='C:\\workspace'):
    cmd = ctypes.create_unicode_buffer('cmd.exe /c exit 42')
    si = STARTUPINFOW()
    si.cb = ctypes.sizeof(si)
    env_ptr = None
    hold = []
    if mode == 'unibuf':
        buf = ctypes.create_unicode_buffer(min_str)
        hold.append(buf)
        env_ptr = ctypes.cast(buf, ctypes.c_void_p)
    elif mode == 'cp':
        cp = ctypes.c_char_p(min_str.encode('utf-16-le'))
        hold.append(cp)
        env_ptr = ctypes.cast(cp, ctypes.c_void_p)
    elif mode == 'ansi':
        ab = ctypes.create_string_buffer(min_str.encode('mbcs'))
        hold.append(ab)
        env_ptr = ctypes.cast(ab, ctypes.c_void_p)
    elif mode == 'full_unibuf':
        buf = ctypes.create_unicode_buffer(full_str)
        hold.append(buf)
        env_ptr = ctypes.cast(buf, ctypes.c_void_p)
    pi = PI()
    ok = advapi32.CreateProcessWithTokenW(tok, 0, None, cmd, flags, env_ptr, cwd, ctypes.byref(si), ctypes.byref(pi))
    log(label, 'CREATE', ok, 'err', ctypes.get_last_error())
    if ok:
        wait_exit(pi, label)

lt = wintypes.HANDLE()
ok_l = advapi32.LogonUserW('AgentUser', 'DESKTOP-QMAPMFH', 'S4Run!2026', 4, 0, ctypes.byref(lt))
log('LOGON', ok_l, ctypes.get_last_error())

if ok_l:
    spawn_cptw('CPTW_NOENV_BEFORE', lt, 'none', 0)
    granted = False
    try:
        g1 = set_sddl(hwinsta, new_ws, ALL_SI)
        g2 = set_sddl(hdesk, new_ds, ALL_SI)
        granted = g1[0] and g2[0]
        log('GRANT_WINSTA', g1)
        log('GRANT_DESKTOP', g2)
        log('WINSTA_SDDL_AFTER', get_sddl(hwinsta, ALL_SI))
        log('DESKTOP_SDDL_AFTER', get_sddl(hdesk, ALL_SI))
        spawn_cptw('CPTW_NOENV_AFTER', lt, 'none', 0)
        spawn_cptw('CPTW_UNIBUF_ENV', lt, 'unibuf', 0x400)
        spawn_cptw('CPTW_CP_ENV', lt, 'cp', 0x400)
        spawn_cptw('CPTW_ANSI_ENV', lt, 'ansi', 0)
        spawn_cptw('CPTW_FULL_ENV', lt, 'full_unibuf', 0x400)
        if granted and ws_sddl0 and ds_sddl0:
            r1s = set_sddl(hwinsta, ws_sddl0, ALL_SI)
            r2s = set_sddl(hdesk, ds_sddl0, ALL_SI)
            log('RESTORE_WINSTA', r1s)
            log('RESTORE_DESKTOP', r2s)
            spawn_cptw('CPTW_NOENV_RESTORED', lt, 'none', 0)
    finally:
        if granted:
            try:
                set_sddl(hwinsta, ws_sddl0, ALL_SI)
                set_sddl(hdesk, ds_sddl0, ALL_SI)
            except Exception as exc:
                log('RESTORE_FINAL_ERR', str(exc))
log('DONE')
"@
Set-Content -LiteralPath 'C:\s4\tools\repro-v4.py' -Value $pyCode -Encoding utf8
& $py 'C:\s4\tools\repro-v4.py' 2>&1 | ForEach-Object { Write-Output "PY: $_" }
'@

$bodyFile = 'D:\CLI\_windows_high_nist\job-repro-v4-body.ps1'
Set-Content -LiteralPath $bodyFile -Value $body -Encoding utf8
try {
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $runner `
        -JobBodyFile $bodyFile -OutFile $OutFile -TimeoutSeconds 400
    if ($LASTEXITCODE -ne 0) { throw "repro-v4 job failed (exit $LASTEXITCODE)" }
}
catch {
    $lines.Add("ERROR: $($_.Exception.Message)")
    $lines | Set-Content -LiteralPath $OutFile -Encoding utf8
}
