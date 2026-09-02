<#
    S4 VM fix machine-wide python install (host-side elevated).
    Diagnoses the mis-targeted install, cleans up, reinstalls to
    C:\Program Files\Python312 with a properly quoted TargetDir, verifies.
    ASCII only.
#>
[CmdletBinding()]
param(
    [string]$OutFile = 'D:\CLI\_windows_high_nist\vm-python-fix-result.txt'
)

$ErrorActionPreference = 'Stop'
$lines = New-Object System.Collections.Generic.List[string]
$runner = 'D:\CLI\scripts\s4_vm_run_elev.ps1'

$body = @'
$env:Path = [Environment]::GetEnvironmentVariable('Path','Machine')
Write-Output "MACHINE_PATH=$env:Path"
foreach ($p in @('C:\Program','C:\Program Files\Python312')) {
    Write-Output "$p exists=$(Test-Path -LiteralPath $p)"
}
Write-Output '--- C:\Program contents ---'
Get-ChildItem -LiteralPath 'C:\Program' -ErrorAction SilentlyContinue | Select-Object -First 30 | ForEach-Object { Write-Output $_.Name }
'@

$bodyFile = 'D:\CLI\_windows_high_nist\job-fix-python-diag-body.ps1'
Set-Content -LiteralPath $bodyFile -Value $body -Encoding utf8
try {
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $runner `
        -JobBodyFile $bodyFile -OutFile $OutFile -TimeoutSeconds 300
    if ($LASTEXITCODE -ne 0) { throw "diag job failed" }
}
catch {
    $lines.Add("ERROR: $($_.Exception.Message)")
    $lines | Set-Content -LiteralPath $OutFile -Encoding utf8
}
