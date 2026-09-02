$env:Path = [Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [Environment]::GetEnvironmentVariable('Path','User')
$env:PYTHONPATH = 'C:\s4'
$run = 'C:\s4\_windows_high_nist\run\run_enforcement_probe.ps1'
New-Item -ItemType Directory -Path 'C:\workspace\control' -Force | Out-Null
Write-Output '===== CONTROL PROBE ====='
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File $run -Arm control -Workspace C:\workspace\control -ResultPath C:\workspace\control\enforcement-probe-control.json -TimeoutSeconds 180 2>&1 | ForEach-Object { Write-Output "CTL: $_" }
$probeExit = $LASTEXITCODE
Write-Output "CTL_PROBE_EXIT=$probeExit"
$obs = 'C:\workspace\control\windows-native-run-observation-control.json'
if (Test-Path -LiteralPath $obs) {
    $o = Get-Content -LiteralPath $obs -Raw | ConvertFrom-Json
    Write-Output "CTL_OBS outcome=$($o.outcome) exit=$($o.process.exit_code)"
    if ($o.diagnostics) { $o.diagnostics | ForEach-Object { Write-Output "CTL_DIAG: $_" } }
    if ($o.checks) { $o.checks.PSObject.Properties | ForEach-Object { Write-Output "CTL_CHECK $($_.Name)=$($_.Value)" } }
}
if (Test-Path -LiteralPath 'C:\workspace\control\enforcement-probe-control.json') {
    Get-Content -LiteralPath 'C:\workspace\control\enforcement-probe-control.json' -Raw
}
exit $probeExit
