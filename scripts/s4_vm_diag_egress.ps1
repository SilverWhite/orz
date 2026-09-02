<#
    S4 VM egress-wall creation diagnostic (host-side elevated). ASCII only.
#>
[CmdletBinding()]
param(
    [string]$OutFile = 'D:\CLI\_windows_high_nist\vm-diag-egress-result.txt'
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
    'import ctypes',
    'from assurance.windows_sandbox import _derive_appcontainer_sid, _appcontainer_sid_to_string, _create_firewall_outbound_block_rule, _is_elevated',
    'print("ELEVATED", _is_elevated())',
    'app = "p2_egress_test_" + "0" * 16',
    'sid = _derive_appcontainer_sid(app)',
    'print("SID_DERIVED", sid is not None)',
    's = _appcontainer_sid_to_string(sid)',
    'print("SID_STRING", s)',
    'name, created, diag = _create_firewall_outbound_block_rule(app, s)',
    'print("RULE_NAME", name)',
    'print("RULE_CREATED", created)',
    'print("DIAG", diag)'
)
$code = $codeLines -join [Environment]::NewLine
$code | Out-File -Encoding utf8 C:\s4\tools\diag-egress.py
& $py C:\s4\tools\diag-egress.py 2>&1 | ForEach-Object { Write-Output "PY: $_" }
'@

$bodyFile = 'D:\CLI\_windows_high_nist\job-diag-egress-body.ps1'
Set-Content -LiteralPath $bodyFile -Value $body -Encoding utf8
try {
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $runner `
        -JobBodyFile $bodyFile -OutFile $OutFile -TimeoutSeconds 300
    if ($LASTEXITCODE -ne 0) { throw "diag-egress job failed (exit $LASTEXITCODE)" }
}
catch {
    $lines.Add("ERROR: $($_.Exception.Message)")
    $lines | Set-Content -LiteralPath $OutFile -Encoding utf8
}
