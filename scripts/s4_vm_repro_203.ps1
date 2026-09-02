<#
    S4 VM WinError 203 reproduction (host-side elevated). ASCII only.
#>
[CmdletBinding()]
param(
    [string]$OutFile = 'D:\CLI\_windows_high_nist\vm-repro-203-result.txt'
)

$ErrorActionPreference = 'Stop'
$lines = New-Object System.Collections.Generic.List[string]
$runner = 'D:\CLI\scripts\s4_vm_run_elev.ps1'

$body = @'
$env:PYTHONIOENCODING = 'utf-8'
$env:Path = [Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [Environment]::GetEnvironmentVariable('Path','User')
$env:PYTHONPATH = 'C:\s4'
$py = 'C:\Program Files\Python312\python.exe'
$codeLines = @(
    'import ctypes, os, re',
    'from ctypes import wintypes',
    'advapi32 = ctypes.WinDLL("advapi32", use_last_error=True)',
    'kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)',
    'advapi32.LogonUserW.argtypes = [wintypes.LPCWSTR, wintypes.LPCWSTR, wintypes.LPCWSTR, wintypes.DWORD, wintypes.DWORD, ctypes.POINTER(wintypes.HANDLE)]',
    'advapi32.LogonUserW.restype = wintypes.BOOL',
    'tok = wintypes.HANDLE()',
    'ok = advapi32.LogonUserW("AgentUser", "DESKTOP-QMAPMFH", "S4Run!2026", 4, 0, ctypes.byref(tok))',
    'print("LOGON", ok, ctypes.get_last_error())',
    'class SI: pass',
    'class PI(ctypes.Structure):',
    '    _fields_ = [("hProcess", wintypes.HANDLE), ("hThread", wintypes.HANDLE), ("dwProcessId", wintypes.DWORD), ("dwThreadId", wintypes.DWORD)]',
    'def spawn(label, env_bytes):',
    '    advapi32.CreateProcessAsUserW.argtypes = [wintypes.HANDLE, wintypes.LPCWSTR, wintypes.LPWSTR, ctypes.c_void_p, ctypes.c_void_p, wintypes.BOOL, wintypes.DWORD, ctypes.c_void_p, wintypes.LPCWSTR, ctypes.c_void_p, ctypes.POINTER(PI)]',
    '    advapi32.CreateProcessAsUserW.restype = wintypes.BOOL',
    '    cmd = ctypes.create_unicode_buffer("cmd.exe /c exit 42")',
    '    env_ptr = ctypes.cast(ctypes.c_char_p(env_bytes), ctypes.c_void_p) if env_bytes is not None else None',
    '    pi = PI()',
    '    class STARTUPINFOW(ctypes.Structure):',
    '        _fields_ = [("cb", wintypes.DWORD), ("lpReserved", wintypes.LPWSTR), ("lpDesktop", wintypes.LPWSTR), ("lpTitle", wintypes.LPWSTR), ("dwX", wintypes.DWORD), ("dwY", wintypes.DWORD), ("dwXSize", wintypes.DWORD), ("dwYSize", wintypes.DWORD), ("dwXCountChars", wintypes.DWORD), ("dwYCountChars", wintypes.DWORD), ("dwFillAttribute", wintypes.DWORD), ("dwFlags", wintypes.DWORD), ("wShowWindow", wintypes.WORD), ("cbReserved2", wintypes.WORD), ("lpReserved2", ctypes.c_void_p), ("hStdInput", wintypes.HANDLE), ("hStdOutput", wintypes.HANDLE), ("hStdError", wintypes.HANDLE)]',
    '    si = STARTUPINFOW()',
    '    si.cb = ctypes.sizeof(si)',
    '    ok2 = advapi32.CreateProcessAsUserW(tok, None, cmd, None, None, True, 0, env_ptr, "C:\\s4\\diag-ws", ctypes.byref(si), ctypes.byref(pi))',
    '    print(label, "CREATE", ok2, "err", ctypes.get_last_error())',
    '    if ok2:',
    '        kernel32.WaitForSingleObject.argtypes = [wintypes.HANDLE, wintypes.DWORD]',
    '        kernel32.WaitForSingleObject.restype = wintypes.DWORD',
    '        kernel32.WaitForSingleObject(pi.hProcess, 15000)',
    '        code = wintypes.DWORD()',
    '        kernel32.GetExitCodeProcess(pi.hProcess, ctypes.byref(code))',
    '        print(label, "EXIT", code.value)',
    '        kernel32.CloseHandle(pi.hProcess)',
    '        kernel32.CloseHandle(pi.hThread)',
    'def build_env():',
    '    items = []',
    '    for k, v in os.environ.items():',
    '        if not k or "=" in k or "\\x00" in k: continue',
    '        if "\\x00" in str(v): continue',
    '        items.append(f"{k}={v}")',
    '    return ("\\x00".join(items) + "\\x00\\x00").encode("utf-16-le")',
    'kernel32.CreateProcessW.argtypes = [wintypes.LPCWSTR, wintypes.LPWSTR, ctypes.c_void_p, ctypes.c_void_p, wintypes.BOOL, wintypes.DWORD, ctypes.c_void_p, wintypes.LPCWSTR, ctypes.c_void_p, ctypes.POINTER(PI)]',
    'kernel32.CreateProcessW.restype = wintypes.BOOL',
    'class STARTUPINFOW2(ctypes.Structure):',
    '    _fields_ = [("cb", wintypes.DWORD), ("lpReserved", wintypes.LPWSTR), ("lpDesktop", wintypes.LPWSTR), ("lpTitle", wintypes.LPWSTR), ("dwX", wintypes.DWORD), ("dwY", wintypes.DWORD), ("dwXSize", wintypes.DWORD), ("dwYSize", wintypes.DWORD), ("dwXCountChars", wintypes.DWORD), ("dwYCountChars", wintypes.DWORD), ("dwFillAttribute", wintypes.DWORD), ("dwFlags", wintypes.DWORD), ("wShowWindow", wintypes.WORD), ("cbReserved2", wintypes.WORD), ("lpReserved2", ctypes.c_void_p), ("hStdInput", wintypes.HANDLE), ("hStdOutput", wintypes.HANDLE), ("hStdError", wintypes.HANDLE)]',
    'def cw(label, env_bytes):',
    '    cmd = ctypes.create_unicode_buffer("cmd.exe /c exit 42")',
    '    env_ptr = ctypes.cast(ctypes.c_char_p(env_bytes), ctypes.c_void_p) if env_bytes is not None else None',
    '    si = STARTUPINFOW2()',
    '    si.cb = ctypes.sizeof(si)',
    '    pi = PI()',
    '    ok = kernel32.CreateProcessW(None, cmd, None, None, True, 0, env_ptr, "C:\\\\s4\\\\diag-ws", ctypes.byref(si), ctypes.byref(pi))',
    '    print(label, "CW", ok, "err", ctypes.get_last_error())',
    '    if ok:',
    '        kernel32.WaitForSingleObject(pi.hProcess, 15000)',
    '        code = wintypes.DWORD()',
    '        kernel32.GetExitCodeProcess(pi.hProcess, ctypes.byref(code))',
    '        print(label, "EXIT", code.value)',
    '        kernel32.CloseHandle(pi.hProcess)',
    '        kernel32.CloseHandle(pi.hThread)',
    'def build_env_from(env):',
    '    items = []',
    '    for k, v in env.items():',
    '        if not k or "=" in k or "\\x00" in k: continue',
    '        if "\\x00" in str(v): continue',
    '        items.append(f"{k}={v}")',
    '    return ("\\x00".join(items) + "\\x00\\x00").encode("utf-16-le")',
    'cw("CW_NOENV", None)',
    'cw("CW_MINENV", b"PATH=" + os.environ.get("PATH", "").encode("utf-16-le") + b"\\x00\\x00")',
    'def cw2(label, env_wstr):',
    '    cmd = ctypes.create_unicode_buffer("cmd.exe /c exit 42")',
    '    buf = ctypes.create_unicode_buffer(env_wstr)',
    '    print(label, "BUF_BYTES", ctypes.string_at(buf, ctypes.sizeof(buf)))',
    '    pi = PI()',
    '    si = STARTUPINFOW2()',
    '    si.cb = ctypes.sizeof(si)',
    '    ok = kernel32.CreateProcessW(None, cmd, None, None, True, 0, ctypes.cast(buf, ctypes.c_void_p), "C:\\\\s4\\\\diag-ws", ctypes.byref(si), ctypes.byref(pi))',
    '    print(label, "CW2", ok, "err", ctypes.get_last_error())',
    '    if ok:',
    '        kernel32.WaitForSingleObject(pi.hProcess, 15000)',
    '        code = wintypes.DWORD()',
    '        kernel32.GetExitCodeProcess(pi.hProcess, ctypes.byref(code))',
    '        print(label, "EXIT", code.value)',
    '        kernel32.CloseHandle(pi.hProcess)',
    '        kernel32.CloseHandle(pi.hThread)',
    'cw2("CW2_UNICODEBUF", "PATH=" + os.environ.get("PATH", "") + "\\x00\\x00")',
    'import subprocess',
    'r = subprocess.run(["cmd.exe", "/c", "exit", "42"], env={"PATH": os.environ.get("PATH", "")}, capture_output=True)',
    'print("SUBPROCESS_MINENV", r.returncode)',
    'r2 = subprocess.run(["cmd.exe", "/c", "exit", "42"], env=os.environ, capture_output=True)',
    'print("SUBPROCESS_FULLENV", r2.returncode)',
    'full = build_env_from(os.environ)',
    'print("CW_FULLENV_LEN", len(full))',
    'cw("CW_FULLENV", full)',
    'bad = [k for k in os.environ if not re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", k)]',
    'print("BAD_KEYS", bad)',
    'good = {k: v for k, v in os.environ.items() if re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", k)}',
    'cw("CW_GOODKEYS", build_env_from(good))'
)
$code = $codeLines -join [Environment]::NewLine
$code | Out-File -Encoding utf8 C:\s4\tools\repro-203.py
& $py C:\s4\tools\repro-203.py 2>&1 | ForEach-Object { Write-Output "PY: $_" }
'@

$bodyFile = 'D:\CLI\_windows_high_nist\job-repro-203-body.ps1'
Set-Content -LiteralPath $bodyFile -Value $body -Encoding utf8
try {
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $runner `
        -JobBodyFile $bodyFile -OutFile $OutFile -TimeoutSeconds 300
    if ($LASTEXITCODE -ne 0) { throw "repro-203 job failed (exit $LASTEXITCODE)" }
}
catch {
    $lines.Add("ERROR: $($_.Exception.Message)")
    $lines | Set-Content -LiteralPath $OutFile -Encoding utf8
}
