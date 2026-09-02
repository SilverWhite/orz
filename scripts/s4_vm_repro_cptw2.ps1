<#
    S4 VM CreateProcessWithTokenW token-type comparison (host-side elevated).
    Combos: {LogonUser token, DuplicateTokenEx primary} x {no env, env},
    plus a primary-token + env + STARTUPINFOEX (job list) combo mirroring the
    real sandbox code path (minus AppContainer).
    ASCII only.
#>
[CmdletBinding()]
param(
    [string]$OutFile = 'D:\CLI\_windows_high_nist\vm-repro-cptw2-result.txt'
)

$ErrorActionPreference = 'Stop'
$lines = New-Object System.Collections.Generic.List[string]
$runner = 'D:\CLI\scripts\s4_vm_run_elev.ps1'

# Wait for the guest to be reachable before creating the job.
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
$codeLines = @(
    'import ctypes, os',
    'from ctypes import wintypes',
    'advapi32 = ctypes.WinDLL("advapi32", use_last_error=True)',
    'kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)',
    'advapi32.LogonUserW.argtypes = [wintypes.LPCWSTR, wintypes.LPCWSTR, wintypes.LPCWSTR, wintypes.DWORD, wintypes.DWORD, ctypes.POINTER(wintypes.HANDLE)]',
    'advapi32.LogonUserW.restype = wintypes.BOOL',
    'lt = wintypes.HANDLE()',
    'ok = advapi32.LogonUserW("AgentUser", "DESKTOP-QMAPMFH", "S4Run!2026", 4, 0, ctypes.byref(lt))',
    'print("LOGON", ok, ctypes.get_last_error())',
    'class PI(ctypes.Structure):',
    '    _fields_ = [("hProcess", wintypes.HANDLE), ("hThread", wintypes.HANDLE), ("dwProcessId", wintypes.DWORD), ("dwThreadId", wintypes.DWORD)]',
    'class STARTUPINFOW(ctypes.Structure):',
    '    _fields_ = [("cb", wintypes.DWORD), ("lpReserved", wintypes.LPWSTR), ("lpDesktop", wintypes.LPWSTR), ("lpTitle", wintypes.LPWSTR), ("dwX", wintypes.DWORD), ("dwY", wintypes.DWORD), ("dwXSize", wintypes.DWORD), ("dwYSize", wintypes.DWORD), ("dwXCountChars", wintypes.DWORD), ("dwYCountChars", wintypes.DWORD), ("dwFillAttribute", wintypes.DWORD), ("dwFlags", wintypes.DWORD), ("wShowWindow", wintypes.WORD), ("cbReserved2", wintypes.WORD), ("lpReserved2", ctypes.c_void_p), ("hStdInput", wintypes.HANDLE), ("hStdOutput", wintypes.HANDLE), ("hStdError", wintypes.HANDLE)]',
    'class STARTUPINFOEX(ctypes.Structure):',
    '    _fields_ = [("StartupInfo", STARTUPINFOW), ("lpAttributeList", ctypes.c_void_p)]',
    'advapi32.CreateProcessWithTokenW.argtypes = [wintypes.HANDLE, wintypes.DWORD, wintypes.LPCWSTR, wintypes.LPWSTR, wintypes.DWORD, ctypes.c_void_p, wintypes.LPCWSTR, ctypes.c_void_p, ctypes.POINTER(PI)]',
    'advapi32.CreateProcessWithTokenW.restype = wintypes.BOOL',
    'advapi32.DuplicateTokenEx.argtypes = [wintypes.HANDLE, wintypes.DWORD, ctypes.c_void_p, wintypes.DWORD, wintypes.DWORD, ctypes.POINTER(wintypes.HANDLE)]',
    'advapi32.DuplicateTokenEx.restype = wintypes.BOOL',
    'pt = wintypes.HANDLE()',
    'ok3 = advapi32.DuplicateTokenEx(lt, 0x000F01FF, None, 2, 1, ctypes.byref(pt))',
    'print("DUP_PRIMARY", ok3, ctypes.get_last_error())',
    'envb = ("PATH=" + os.environ.get("PATH", "") + "\\x00\\x00").encode("utf-16-le")',
    'ep = ctypes.cast(ctypes.c_char_p(envb), ctypes.c_void_p)',
    'def spawn(label, tok, env_ptr, use_ex=False):',
    '    cmd = ctypes.create_unicode_buffer("cmd.exe /c exit 42")',
    '    flags = 0x00000400 if env_ptr is not None else 0',
    '    if use_ex:',
    '        flags |= 0x00080000  # EXTENDED_STARTUPINFO_PRESENT',
    '        kernel32.InitializeProcThreadAttributeList.argtypes = [ctypes.c_void_p, wintypes.DWORD, wintypes.DWORD, ctypes.POINTER(ctypes.c_size_t)]',
    '        kernel32.InitializeProcThreadAttributeList.restype = wintypes.BOOL',
    '        sz = ctypes.c_size_t()',
    '        kernel32.InitializeProcThreadAttributeList(None, 1, 0, ctypes.byref(sz))',
    '        buf = ctypes.create_string_buffer(sz.value)',
    '        kernel32.InitializeProcThreadAttributeList(ctypes.cast(buf, ctypes.c_void_p), 1, 0, ctypes.byref(sz))',
    '        si = STARTUPINFOEX()',
    '        si.StartupInfo.cb = ctypes.sizeof(STARTUPINFOEX)',
    '        si.lpAttributeList = ctypes.cast(buf, ctypes.c_void_p)',
    '        si_ptr = ctypes.cast(ctypes.byref(si), ctypes.c_void_p)',
    '    else:',
    '        si = STARTUPINFOW()',
    '        si.cb = ctypes.sizeof(si)',
    '        si_ptr = ctypes.cast(ctypes.byref(si), ctypes.c_void_p)',
    '    pi = PI()',
    '    ok2 = advapi32.CreateProcessWithTokenW(tok, 0, None, cmd, flags, env_ptr, "C:\\\\s4\\\\diag-ws", si_ptr, ctypes.byref(pi))',
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
    'spawn("LOGON_NOENV", lt, None)',
    'spawn("PRIMARY_NOENV", pt, None)',
    'spawn("LOGON_ENV", lt, ep)',
    'spawn("PRIMARY_ENV", pt, ep)',
    'spawn("PRIMARY_ENV_EX", pt, ep, use_ex=True)'
)
$code = $codeLines -join [Environment]::NewLine
$code | Out-File -Encoding utf8 C:\s4\tools\repro-cptw.py
& $py C:\s4\tools\repro-cptw.py 2>&1 | ForEach-Object { Write-Output "PY: $_" }
'@

$bodyFile = 'D:\CLI\_windows_high_nist\job-repro-cptw2-body.ps1'
Set-Content -LiteralPath $bodyFile -Value $body -Encoding utf8
try {
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $runner `
        -JobBodyFile $bodyFile -OutFile $OutFile -TimeoutSeconds 300
    if ($LASTEXITCODE -ne 0) { throw "repro-cptw2 job failed (exit $LASTEXITCODE)" }
}
catch {
    $lines.Add("ERROR: $($_.Exception.Message)")
    $lines | Set-Content -LiteralPath $OutFile -Encoding utf8
}
