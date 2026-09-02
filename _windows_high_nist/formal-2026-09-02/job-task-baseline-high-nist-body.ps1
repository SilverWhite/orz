$env:Path = [Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [Environment]::GetEnvironmentVariable('Path','User')
$env:PYTHONPATH = 'C:\s4'
$hard = 'C:\s4\_windows_high_nist\hardening\apply_hardening.ps1'
$probeRun = 'C:\s4\_windows_high_nist\run\run_enforcement_probe.ps1'
$taskRun = 'C:\s4\_windows_high_nist\run\run_task_arm.ps1'
$ws = 'C:\workspace\taskhighnist'
New-Item -ItemType Directory -Path $ws -Force | Out-Null
Write-Output '===== APPLY HIGH-NIST (DeepSeek pin + AppLocker restore) ====='
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File $hard -Arm high-nist -RunUser AgentUser -WorkspaceRoot C:\workspace -DeepSeekIp 221.204.163.76 -EnableAppLocker 2>&1 | ForEach-Object { Write-Output "THNH: $_" }
Write-Output "THNH_EXIT=$LASTEXITCODE"
if ($LASTEXITCODE -ne 0) {
    Write-Output 'TASKHIGHNIST_APPLY_FAILED'
    exit 1
}
Write-Output '===== HIGH-NIST WALL PROBE ====='
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File $probeRun -Arm high-nist -Workspace $ws -ResultPath (Join-Path $ws 'enforcement-probe-high-nist.json') -TimeoutSeconds 180 2>&1 | ForEach-Object { Write-Output "THNP: $_" }
$probeExit = $LASTEXITCODE
Write-Output "THN_PROBE_EXIT=$probeExit"
if ($probeExit -ne 0) {
    Write-Output 'TASKHIGHNIST_PROBE_FAILED'
    exit 1
}
Write-Output '===== HIGH-NIST TASK BATCH ====='
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File $taskRun -Arm high-nist -Workspace $ws -ResultPath (Join-Path $ws 'task-baseline-high-nist.json') 2>&1 | ForEach-Object { Write-Output "THNT: $_" }
$taskExit = $LASTEXITCODE
Write-Output "TASK_HIGHNIST_EXIT=$taskExit"
if (Test-Path -LiteralPath (Join-Path $ws 'task-baseline-high-nist.json')) {
    Get-Content -LiteralPath (Join-Path $ws 'task-baseline-high-nist.json') -Raw
}
exit $taskExit
