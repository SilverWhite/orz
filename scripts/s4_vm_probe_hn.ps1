<#
    S4 VM high-nist probe with persistent workspace (host-side elevated).
    ASCII only.
#>
[CmdletBinding()]
param(
    [string]$OutFile = 'D:\CLI\_windows_high_nist\vm-probe-hn-result.txt'
)

$ErrorActionPreference = 'Stop'
$lines = New-Object System.Collections.Generic.List[string]
$runner = 'D:\CLI\scripts\s4_vm_run_elev.ps1'

$body = @'
$env:Path = [Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [Environment]::GetEnvironmentVariable('Path','User')
New-Item -ItemType Directory -Path 'C:\s4\results\ws-hn' -Force | Out-Null
& C:\s4\_windows_high_nist\run\run_enforcement_probe.ps1 -Arm high-nist -Workspace C:\s4\results\ws-hn -ResultPath C:\s4\results\enforcement-probe-high-nist.json
Write-Output "PROBE_EXIT=$LASTEXITCODE"
$obs = 'C:\s4\results\ws-hn\windows-native-run-observation-high-nist.json'
if (Test-Path -LiteralPath $obs) {
    $j = Get-Content -LiteralPath $obs -Raw | ConvertFrom-Json
    Write-Output "OBS outcome=$($j.outcome) exit=$($j.process.exit_code)"
    $j.checks.PSObject.Properties | ForEach-Object { Write-Output "CHECK $($_.Name)=$($_.Value)" }
    if ($j.diagnostics) {
        Write-Output '--- diagnostics ---'
        $j.diagnostics | ForEach-Object { Write-Output "DIAG: $_" }
    }
}
'@

$bodyFile = 'D:\CLI\_windows_high_nist\job-probe-hn-body.ps1'
Set-Content -LiteralPath $bodyFile -Value $body -Encoding utf8
try {
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $runner `
        -JobBodyFile $bodyFile -OutFile $OutFile -TimeoutSeconds 400
    if ($LASTEXITCODE -ne 0) { throw "probe-hn job failed (exit $LASTEXITCODE)" }
}
catch {
    $lines.Add("ERROR: $($_.Exception.Message)")
    $lines | Set-Content -LiteralPath $OutFile -Encoding utf8
}
