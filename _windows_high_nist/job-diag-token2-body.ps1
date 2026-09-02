$py = 'C:\Program Files\Python312\python.exe'
$env:PYTHONPATH = 'C:\s4'
$codeLines = @(
    'from assurance.windows_sandbox import _create_restricted_token, build_restricted_token_spec',
    'from assurance.windows_sandbox import _duplicate_token_primary, _token_has_enabled_group, _token_enabled_privileges',
    'import ctypes',
    'd = []',
    'spec = build_restricted_token_spec("non-admin")',
    't = _create_restricted_token(spec, diagnostics=d)',
    'print("TOKEN", t is not None)',
    'print("DIAGS", d)',
    'if t:',
    '    p = _duplicate_token_primary(t, diagnostics=d)',
    '    print("PRIMARY", p is not None)',
    '    if p:',
    '        print("ADMIN_ENABLED", _token_has_enabled_group(p, "S-1-5-32-544", diagnostics=d))',
    '        print("ENABLED_PRIVS", _token_enabled_privileges(p, diagnostics=d))',
    '        ctypes.windll.kernel32.CloseHandle(p)',
    '    ctypes.windll.kernel32.CloseHandle(t)'
)
$code = $codeLines -join [Environment]::NewLine
$code | Out-File -Encoding utf8 C:\s4\tools\diag-token2.py
& $py C:\s4\tools\diag-token2.py 2>&1 | ForEach-Object { Write-Output "PY: $_" }
