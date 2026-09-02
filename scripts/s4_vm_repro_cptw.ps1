<#
    S4 VM CreateProcessWithTokenW token-type comparison (host-side elevated).
    ASCII only.
#>
[CmdletBinding()]
param(
    [string]$OutFile = 'D:\CLI\_windows_high_nist\vm-repro-cptw-result.txt'
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
    'import ctypes, os',
    'from ctypes import wintypes',
    'advapi32 = ctypes.WinDLL("advapi32", use_last_error=True)',
    'kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)',
    'advapi32.LogonUserW.argtypes = [wintypes.LPCWSTR, wintypes.LPCWSTR, wintypes.LPCWSTR, wintypes.DWORD, wintypes.DWORD, ctypes.POINTER(wintypes.HANDLE)]',
    'advapi32.LogonUserW.restype = wintypes.BOOL',
    'lt = wintypes.HANDLE()',
    'ok = advapi32.LogonUserW("AgentUser", "DESKTOP-QMAPMFH", "S4Run!2026", 4, 0, ctypes.byref(lt))',
    'print("LOGON", ok)',
    'class PI(ctypes.Structure):',
    '    _fields_ = [("hProcess", wintypes.HANDLE), ("hThread", wintypes.HANDLE), ("dwProcessId", wintypes.DWORD), ("dwThreadId", wintypes.DWORD)]',
    'class STARTUPINFOW(ctypes.Structure):',
    '    _fields_ = [("cb", wintypes.DWORD), ("lpReserved", wintypes.LPWSTR), ("lpDesktop", wintypes.LPWSTR), ("lpTitle", wintypes.LPWSTR), ("dwX", wintypes.DWORD), ("dwY", wintypes.DWORD), ("dwXSize", wintypes.DWORD), ("dwYSize", wintypes.DWORD), ("dwXCountChars", wintypes.DWORD), ("dwYCountChars", wintypes.DWORD), ("dwFillAttribute", wintypes.DWORD), ("dwFlags", wintypes.DWORD), ("wShowWindow", wintypes.WORD), ("cbReserved2", wintypes.WORD), ("lpReserved2", ctypes.c_void_p), ("hStdInput", wintypes.HANDLE), ("hStdOutput", wintypes.HANDLE), ("hStdError", wintypes.HANDLE)]',
    'advapi32.CreateProcessWithTokenW.argtypes = [wintypes.HANDLE, wintypes.DWORD, wintypes.LPCWSTR, wintypes.LPWSTR, wintypes.DWORD, ctypes.c_void_p, wintypes.LPCWSTR, ctypes.c_void_p, ctypes.POINTER(PI)]',
    'advapi32.CreateProcessWithTokenW.restype = wintypes.BOOL',
    'def spawn(label, tok, env_ptr):',
    '    cmd = ctypes.create_unicode_buffer("cmd.exe /c exit 42")',
    '    si = STARTUPINFOW()',
    '    si.cb = ctypes.sizeof(si)',
    '    pi = PI()',
    '    flags = 0x00000400 if env_ptr is not None else 0',
    '    ok2 = advapi32.CreateProcessWithTokenW(tok, 0, None, cmd, flags, env_ptr, "C:\\\\s4\\\\diag-ws", ctypes.byref(si), ctypes.byref(pi))',
    '    print(label, ok2, ctypes.get_last_error())',
    '    if ok2:',
    '        kernel32.WaitForSingleObject(pi.hProcess, 15000)',
    '        code = wintypes.DWORD()',
    '        kernel32.GetExitCodeProcess(pi.hProcess, ctypes.byref(code))',
    '        print(label, "EXIT", code.value)',
    'spawn("LOGON_NOENV", lt, None)',
    'advapi32.DuplicateTokenEx.argtypes = [wintypes.HANDLE, wintypes.DWORD, ctypes.c_void_p, wintypes.DWORD, wintypes.DWORD, ctypes.POINTER(wintypes.HANDLE)]',
    'advapi32.DuplicateTokenEx.restype = wintypes.BOOL',
    'pt = wintypes.HANDLE()',
    'ok3 = advapi32.DuplicateTokenEx(lt, 0x000F01FF, None, 2, 1, ctypes.byref(pt))',
    'print("DUP_PRIMARY", ok3)',
    'spawn("PRIMARY_NOENV", pt, None)',
    'envb = ("PATH=" + os.environ.get("PATH", "") + "\\x00\\x00").encode("utf-16-le")',
    'ep = ctypes.cast(ctypes.c_char_p(envb), ctypes.c_void_p)',
    'spawn("LOGON_ENV", lt, ep)'
)
$code = $codeLines -join [Environment]::NewLine
$code | Out-File -Encoding utf8 C:\s4\tools\repro-cptw.py
& $py C:\s4\tools\repro-cptw.py 2>&1 | ForEach-Object { Write-Output "PY: $_" }
'@

$bodyFile = 'D:\CLI\_windows_high_nist\job-repro-cptw-body.ps1'
Set-Content -LiteralPath $bodyFile -Value $body -Encoding utf8
try {
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $runner `
        -JobBodyFile $bodyFile -OutFile $OutFile -TimeoutSeconds 300
    if ($LASTEXITCODE -ne 0) { throw "repro-cptw job failed (exit $LASTEXITCODE)" }
}
catch {
    $lines.Add("ERROR: $($_.Exception.Message)")
    $lines | Set-Content -LiteralPath $OutFile -Encoding utf8
}
