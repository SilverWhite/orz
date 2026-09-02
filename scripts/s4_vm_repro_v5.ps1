<#
    S4 VM repro v5: binary search for the 87 in the sandbox's
    CreateProcessWithTokenW path.  Starts from an exact replica of the
    sandbox call, then strips one feature at a time:
      A exact (virt-off + pipes + EXTENDED/job + SUSPEND + NOWINDOW + env)
      B no pipes
      C no virt-off
      D no EXTENDED/attr-list
      E no SUSPEND
      F no virt-off + no pipes
      G exact + LOW integrity (high-nist replica)
    Desktop grant is applied before all variants (reuses v4 logic).
    ASCII only.
#>
[CmdletBinding()]
param(
    [string]$OutFile = 'D:\CLI\_windows_high_nist\vm-repro-v5-result.txt'
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
New-Item -ItemType Directory -Path 'C:\workspace' -Force | Out-Null
$pyCode = @"
import ctypes, os
from ctypes import wintypes

def log(*a):
    print(' '.join(str(x) for x in a), flush=True)

kernel32 = ctypes.WinDLL('kernel32', use_last_error=True)
advapi32 = ctypes.WinDLL('advapi32', use_last_error=True)
user32 = ctypes.WinDLL('user32', use_last_error=True)

EXTENDED = 0x00080000
SUSPENDED = 0x00000004
NO_WINDOW = 0x08000000
NEW_GROUP = 0x00000200
UNICODE_ENV = 0x00000400

# --- desktop grant (from v4, trimmed) ---
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
granted = False
try:
    g1 = set_sddl(hwinsta, add_ace(ws0, agent_sid))
    g2 = set_sddl(hdesk, add_ace(ds0, agent_sid))
    granted = g1 and g2
    log('GRANT', g1, g2)
finally:
    pass

class STARTUPINFOW(ctypes.Structure):
    _fields_ = [('cb', wintypes.DWORD), ('lpReserved', wintypes.LPWSTR), ('lpDesktop', wintypes.LPWSTR), ('lpTitle', wintypes.LPWSTR), ('dwX', wintypes.DWORD), ('dwY', wintypes.DWORD), ('dwXSize', wintypes.DWORD), ('dwYSize', wintypes.DWORD), ('dwXCountChars', wintypes.DWORD), ('dwYCountChars', wintypes.DWORD), ('dwFillAttribute', wintypes.DWORD), ('dwFlags', wintypes.DWORD), ('wShowWindow', wintypes.WORD), ('cbReserved2', wintypes.WORD), ('lpReserved2', ctypes.POINTER(wintypes.BYTE)), ('hStdInput', wintypes.HANDLE), ('hStdOutput', wintypes.HANDLE), ('hStdError', wintypes.HANDLE)]

class STARTUPINFOEX(ctypes.Structure):
    _fields_ = [('StartupInfo', STARTUPINFOW), ('lpAttributeList', ctypes.c_void_p)]

class PI(ctypes.Structure):
    _fields_ = [('hProcess', wintypes.HANDLE), ('hThread', wintypes.HANDLE), ('dwProcessId', wintypes.DWORD), ('dwThreadId', wintypes.DWORD)]

advapi32.LogonUserW.argtypes = [wintypes.LPCWSTR, wintypes.LPCWSTR, wintypes.LPCWSTR, wintypes.DWORD, wintypes.DWORD, ctypes.POINTER(wintypes.HANDLE)]
advapi32.LogonUserW.restype = wintypes.BOOL
advapi32.CreateProcessWithTokenW.argtypes = [wintypes.HANDLE, wintypes.DWORD, wintypes.LPCWSTR, wintypes.LPWSTR, wintypes.DWORD, ctypes.c_void_p, wintypes.LPCWSTR, ctypes.c_void_p, ctypes.POINTER(PI)]
advapi32.CreateProcessWithTokenW.restype = wintypes.BOOL

lt = wintypes.HANDLE()
okl = advapi32.LogonUserW('AgentUser', 'DESKTOP-QMAPMFH', 'S4Run!2026', 4, 0, ctypes.byref(lt))
log('LOGON', okl, ctypes.get_last_error())

# token helpers
advapi32.SetTokenInformation.argtypes = [wintypes.HANDLE, ctypes.c_int, ctypes.c_void_p, wintypes.DWORD]
advapi32.SetTokenInformation.restype = wintypes.BOOL

def set_virt(tok, val):
    v = wintypes.BOOL(val)
    return bool(advapi32.SetTokenInformation(tok, 24, ctypes.byref(v), ctypes.sizeof(v)))  # TokenVirtualizationAllowed=24

def set_lowil(tok):
    sid = ctypes.c_void_p()
    auth = (ctypes.c_ubyte * 6)(0, 0, 0, 0, 0, 16)
    advapi32.AllocateAndInitializeSid.argtypes = [ctypes.POINTER(ctypes.c_ubyte * 6), wintypes.DWORD] + [wintypes.DWORD] * 8 + [ctypes.POINTER(ctypes.c_void_p)]
    advapi32.AllocateAndInitializeSid.restype = wintypes.BOOL
    if not advapi32.AllocateAndInitializeSid(auth, 1, 4096, 0, 0, 0, 0, 0, 0, 0, ctypes.byref(sid)):
        return False
    class TOKEN_MANDATORY_LABEL(ctypes.Structure):
        _fields_ = [('Label', type('SAA', (ctypes.Structure,), {'_fields_': [('Sid', ctypes.c_void_p), ('Attributes', wintypes.DWORD)]}))]
    label = TOKEN_MANDATORY_LABEL()
    label.Label.Sid = sid
    label.Label.Attributes = 0x20
    ok = bool(advapi32.SetTokenInformation(tok, 25, ctypes.byref(label), ctypes.sizeof(label)))  # TokenIntegrityLevel=25
    advapi32.FreeSid.argtypes = [ctypes.c_void_p]
    advapi32.FreeSid(sid)
    return ok

# pipes
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

nul = kernel32.CreateFileW('NUL', 0x80000000, 0x3, None, 3, 0, None)

def env_block(env):
    items = []
    for k, v in env.items():
        if not k or '=' in k or '\x00' in k or '\x00' in str(v):
            continue
        items.append(str(k) + '=' + str(v))
    s = '\x00'.join(items) + '\x00\x00'
    return s

full_env = dict(os.environ)
full_env.update({'USERNAME': 'AgentUser', 'USERPROFILE': r'C:\Users\AgentUser', 'TEMP': r'C:\workspace\.tmp', 'TMP': r'C:\workspace\.tmp'})
env_s = env_block(full_env)

def wait_exit(pi, label):
    kernel32.WaitForSingleObject.argtypes = [wintypes.HANDLE, wintypes.DWORD]
    kernel32.WaitForSingleObject.restype = wintypes.DWORD
    kernel32.WaitForSingleObject(pi.hProcess, 15000)
    code = wintypes.DWORD()
    kernel32.GetExitCodeProcess.argtypes = [wintypes.HANDLE, ctypes.POINTER(wintypes.DWORD)]
    kernel32.GetExitCodeProcess.restype = wintypes.BOOL
    kernel32.GetExitCodeProcess(pi.hProcess, ctypes.byref(code))
    log(label, 'EXIT', hex(code.value) if code.value > 0x7fffffff else code.value)
    kernel32.CloseHandle(pi.hProcess)
    kernel32.CloseHandle(pi.hThread)

def spawn(label, use_virt=True, use_pipes=True, use_ext=True, use_suspend=True, use_lowil=False):
    tok = wintypes.HANDLE()
    advapi32.LogonUserW('AgentUser', 'DESKTOP-QMAPMFH', 'S4Run!2026', 4, 0, ctypes.byref(tok))
    if use_virt:
        set_virt(tok, False)
    if use_lowil:
        set_lowil(tok)
    flags = NO_WINDOW | NEW_GROUP | UNICODE_ENV
    if use_suspend:
        flags |= SUSPENDED
    if use_ext:
        flags |= EXTENDED
    eb = ctypes.create_unicode_buffer(env_s)
    env_ptr = ctypes.cast(eb, ctypes.c_void_p)
    si = None
    attr_list = None
    if use_ext:
        kernel32.InitializeProcThreadAttributeList.argtypes = [ctypes.c_void_p, wintypes.DWORD, wintypes.DWORD, ctypes.POINTER(ctypes.c_size_t)]
        kernel32.InitializeProcThreadAttributeList.restype = wintypes.BOOL
        sz = ctypes.c_size_t()
        kernel32.InitializeProcThreadAttributeList(None, 1, 0, ctypes.byref(sz))
        attr_list = ctypes.create_string_buffer(sz.value)
        kernel32.InitializeProcThreadAttributeList(ctypes.cast(attr_list, ctypes.c_void_p), 1, 0, ctypes.byref(sz))
        kernel32.UpdateProcThreadAttribute.argtypes = [ctypes.c_void_p, wintypes.DWORD, ctypes.c_size_t, ctypes.c_void_p, ctypes.c_size_t, ctypes.c_void_p, ctypes.c_void_p]
        kernel32.UpdateProcThreadAttribute.restype = wintypes.BOOL
        kernel32.CreateJobObjectW.argtypes = [ctypes.c_void_p, wintypes.LPCWSTR]
        kernel32.CreateJobObjectW.restype = wintypes.HANDLE
        job = wintypes.HANDLE(kernel32.CreateJobObjectW(None, None))
        kernel32.UpdateProcThreadAttribute(ctypes.cast(attr_list, ctypes.c_void_p), 0, 0x0002000B, ctypes.byref(job), ctypes.sizeof(wintypes.HANDLE), None, None)
        si = STARTUPINFOEX()
        si.StartupInfo.cb = ctypes.sizeof(STARTUPINFOEX)
        si.lpAttributeList = ctypes.cast(attr_list, ctypes.c_void_p)
    else:
        si = STARTUPINFOW()
        si.cb = ctypes.sizeof(si)
    if use_pipes:
        r1, w1 = make_pipes()
        r2, w2 = make_pipes()
        if use_ext:
            si.StartupInfo.dwFlags |= 0x100  # STARTF_USESTDHANDLES
            si.StartupInfo.hStdOutput = w1
            si.StartupInfo.hStdError = w2 or w1
            si.StartupInfo.hStdInput = nul
        else:
            si.dwFlags |= 0x100
            si.hStdOutput = w1
            si.hStdError = w2 or w1
            si.hStdInput = nul
    cmd = ctypes.create_unicode_buffer('cmd.exe /c exit 42')
    pi = PI()
    ok = advapi32.CreateProcessWithTokenW(tok, 0, None, cmd, flags, env_ptr, 'C:\\workspace', ctypes.cast(ctypes.byref(si), ctypes.c_void_p), ctypes.byref(pi))
    log(label, 'CREATE', ok, 'err', ctypes.get_last_error())
    if ok:
        wait_exit(pi, label)
    if attr_list is not None:
        kernel32.DeleteProcThreadAttributeList.argtypes = [ctypes.c_void_p]
        kernel32.DeleteProcThreadAttributeList(ctypes.cast(attr_list, ctypes.c_void_p))
    kernel32.CloseHandle(tok)

spawn('A_EXACT', True, True, True, True)
spawn('B_NO_PIPES', True, False, True, True)
spawn('C_NO_VIRT', False, True, True, True)
spawn('D_NO_EXT', True, True, False, True)
spawn('E_NO_SUSPEND', True, True, True, False)
spawn('F_NO_VIRT_NO_PIPES', False, False, True, True)
spawn('G_PLUS_LOWIL', True, True, True, True, use_lowil=True)

# restore desktop ACLs
set_sddl(hwinsta, ws0)
set_sddl(hdesk, ds0)
log('RESTORED')
log('DONE')
"@
Set-Content -LiteralPath 'C:\s4\tools\repro-v5.py' -Value $pyCode -Encoding utf8
& $py 'C:\s4\tools\repro-v5.py' 2>&1 | ForEach-Object { Write-Output "PY: $_" }
'@

$bodyFile = 'D:\CLI\_windows_high_nist\job-repro-v5-body.ps1'
Set-Content -LiteralPath $bodyFile -Value $body -Encoding utf8
try {
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $runner `
        -JobBodyFile $bodyFile -OutFile $OutFile -TimeoutSeconds 400
    if ($LASTEXITCODE -ne 0) { throw "repro-v5 job failed (exit $LASTEXITCODE)" }
}
catch {
    $lines.Add("ERROR: $($_.Exception.Message)")
    $lines | Set-Content -LiteralPath $OutFile -Encoding utf8
}
