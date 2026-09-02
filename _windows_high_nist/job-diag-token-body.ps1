$py = 'C:\Program Files\Python312\python.exe'
Set-Location -LiteralPath 'C:\s4'
$codeLines = @(
    'import sys',
    'from assurance.windows_sandbox import _string_sid_to_ptr, _create_restricted_token, build_restricted_token_spec',
    'diags = []',
    'p = _string_sid_to_ptr("S-1-5-32-544", diagnostics=diags)',
    'print("SID_PTR", p is not None)',
    'print("DIAGS", diags)',
    'spec = build_restricted_token_spec("non-admin")',
    'print("SPEC", spec)',
    't = _create_restricted_token(spec, diagnostics=diags)',
    'print("TOKEN", t is not None)',
    'print("DIAGS2", diags)'
)
$code = $codeLines -join [Environment]::NewLine
$code | Out-File -Encoding utf8 C:\s4\tools\diag-token.py
$env:PYTHONPATH = 'C:\s4'
& $py C:\s4\tools\diag-token.py 2>&1 | ForEach-Object { Write-Output "PY: $_" }
