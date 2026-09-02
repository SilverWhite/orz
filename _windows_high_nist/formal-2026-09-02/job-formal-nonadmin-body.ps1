$env:Path = [Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [Environment]::GetEnvironmentVariable('Path','User')
$env:PYTHONPATH = 'C:\s4'
$hard = 'C:\s4\_windows_high_nist\hardening\apply_hardening.ps1'
$run = 'C:\s4\_windows_high_nist\run\run_enforcement_probe.ps1'
New-Item -ItemType Directory -Path 'C:\workspace\non-admin' -Force | Out-Null
Write-Output '===== APPLY NON-ADMIN ====='
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File $hard -Arm non-admin -RunUser AgentUser -WorkspaceRoot C:\workspace 2>&1 | ForEach-Object { Write-Output "NAH: $_" }
Write-Output "NAH_EXIT=$LASTEXITCODE"
if ($LASTEXITCODE -ne 0) {
    Write-Output 'NONADMIN_APPLY_FAILED'
    exit 1
}
Write-Output '===== NON-ADMIN PROBE ====='
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File $run -Arm non-admin -Workspace C:\workspace\non-admin -ResultPath C:\workspace\non-admin\enforcement-probe-non-admin.json -TimeoutSeconds 180 2>&1 | ForEach-Object { Write-Output "NA: $_" }
$probeExit = $LASTEXITCODE
Write-Output "NA_PROBE_EXIT=$probeExit"
$obs = 'C:\workspace\non-admin\windows-native-run-observation-non-admin.json'
if (Test-Path -LiteralPath $obs) {
    $o = Get-Content -LiteralPath $obs -Raw | ConvertFrom-Json
    Write-Output "NA_OBS outcome=$($o.outcome) exit=$($o.process.exit_code)"
    if ($o.diagnostics) { $o.diagnostics | ForEach-Object { Write-Output "NA_DIAG: $_" } }
    if ($o.checks) { $o.checks.PSObject.Properties | ForEach-Object { Write-Output "NA_CHECK $($_.Name)=$($_.Value)" } }
}
if (Test-Path -LiteralPath 'C:\workspace\non-admin\enforcement-probe-non-admin.json') {
    Get-Content -LiteralPath 'C:\workspace\non-admin\enforcement-probe-non-admin.json' -Raw
}
exit $probeExit
