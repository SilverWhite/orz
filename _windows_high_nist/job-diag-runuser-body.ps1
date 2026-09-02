$env:PYTHONIOENCODING = 'utf-8'
$env:Path = [Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [Environment]::GetEnvironmentVariable('Path','User')
$env:PYTHONPATH = 'C:\s4'
Write-Output "MACHINE_USER=$([Environment]::GetEnvironmentVariable('ORZ_WINDOWS_RUN_USER','Machine'))"
Write-Output "PROC_USER=$env:ORZ_WINDOWS_RUN_USER"
$py = 'C:\Program Files\Python312\python.exe'
$codeLines = @(
    'import os',
    'print("OS_ENV_USER", os.environ.get("ORZ_WINDOWS_RUN_USER"))',
    'print("OS_ENV_PW_SET", bool(os.environ.get("ORZ_WINDOWS_RUN_USER_PASSWORD")))',
    'from assurance.windows_sandbox import _logon_run_user, _duplicate_token_primary, _is_elevated, _get_run_user_credentials',
    'print("ELEVATED", _is_elevated())',
    'print("CREDS", _get_run_user_credentials())',
    'd = []',
    't = _logon_run_user(("DESKTOP-QMAPMFH", "AgentUser", "S4Run!2026"), diagnostics=d)',
    'print("LOGON", t is not None, d)',
    'if t:',
    '    p = _duplicate_token_primary(t, diagnostics=d)',
    '    print("PRIMARY", p is not None, d)'
)
$code = $codeLines -join [Environment]::NewLine
$code | Out-File -Encoding utf8 C:\s4\tools\diag-runuser.py
& $py C:\s4\tools\diag-runuser.py 2>&1 | ForEach-Object { Write-Output "PY: $_" }
