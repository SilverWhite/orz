<#
    S4 VM restricted-token SID parse diagnostic (host-side elevated).
    ASCII only.
#>
[CmdletBinding()]
param(
    [string]$OutFile = 'D:\CLI\_windows_high_nist\vm-diag-token-result.txt'
)

$ErrorActionPreference = 'Stop'
$lines = New-Object System.Collections.Generic.List[string]
$runner = 'D:\CLI\scripts\s4_vm_run_elev.ps1'

$body = @'
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
'@

$bodyFile = 'D:\CLI\_windows_high_nist\job-diag-token-body.ps1'
Set-Content -LiteralPath $bodyFile -Value $body -Encoding utf8
try {
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $runner `
        -JobBodyFile $bodyFile -OutFile $OutFile -TimeoutSeconds 300
    if ($LASTEXITCODE -ne 0) { throw "diag-token job failed (exit $LASTEXITCODE)" }
}
catch {
    $lines.Add("ERROR: $($_.Exception.Message)")
    $lines | Set-Content -LiteralPath $OutFile -Encoding utf8
}
