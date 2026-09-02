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
