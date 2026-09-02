$env:Path = [Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [Environment]::GetEnvironmentVariable('Path','User')
$env:PYTHONPATH = 'C:\s4'
$hard = 'C:\s4\_windows_high_nist\hardening\apply_hardening.ps1'
$run = 'C:\s4\_windows_high_nist\run\run_enforcement_probe.ps1'
New-Item -ItemType Directory -Path 'C:\workspace\high-nist' -Force | Out-Null
Write-Output '===== REVERT NON-ADMIN ====='
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File $hard -Arm non-admin -Revert -RunUser AgentUser -WorkspaceRoot C:\workspace 2>&1 | ForEach-Object { Write-Output "REVNA: $_" }
Write-Output "REVNA_EXIT=$LASTEXITCODE"
Write-Output '===== APPLY HIGH-NIST (DeepSeek pin + AppLocker restore) ====='
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File $hard -Arm high-nist -RunUser AgentUser -WorkspaceRoot C:\workspace -DeepSeekIp 221.204.163.76 -EnableAppLocker 2>&1 | ForEach-Object { Write-Output "HNH: $_" }
Write-Output "HNH_EXIT=$LASTEXITCODE"
if ($LASTEXITCODE -ne 0) {
    Write-Output 'HIGHNIST_APPLY_FAILED'
    exit 1
}
Write-Output '===== HIGH-NIST PROBE ====='
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File $run -Arm high-nist -Workspace C:\workspace\high-nist -ResultPath C:\workspace\high-nist\enforcement-probe-high-nist.json -TimeoutSeconds 180 2>&1 | ForEach-Object { Write-Output "HN: $_" }
$probeExit = $LASTEXITCODE
Write-Output "HN_PROBE_EXIT=$probeExit"
$obs = 'C:\workspace\high-nist\windows-native-run-observation-high-nist.json'
if (Test-Path -LiteralPath $obs) {
    $o = Get-Content -LiteralPath $obs -Raw | ConvertFrom-Json
    Write-Output "HN_OBS outcome=$($o.outcome) exit=$($o.process.exit_code)"
    if ($o.diagnostics) { $o.diagnostics | ForEach-Object { Write-Output "HN_DIAG: $_" } }
    if ($o.checks) { $o.checks.PSObject.Properties | ForEach-Object { Write-Output "HN_CHECK $($_.Name)=$($_.Value)" } }
}
if (Test-Path -LiteralPath 'C:\workspace\high-nist\enforcement-probe-high-nist.json') {
    Get-Content -LiteralPath 'C:\workspace\high-nist\enforcement-probe-high-nist.json' -Raw
}
exit $probeExit
