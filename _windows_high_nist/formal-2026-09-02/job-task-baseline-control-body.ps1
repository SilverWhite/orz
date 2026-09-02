$env:Path = [Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [Environment]::GetEnvironmentVariable('Path','User')
$env:PYTHONPATH = 'C:\s4'
$run = 'C:\s4\_windows_high_nist\run\run_task_arm.ps1'
New-Item -ItemType Directory -Path 'C:\workspace\taskcontrol' -Force | Out-Null
Write-Output '===== TASK CONTROL BASELINE ====='
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File $run -Arm control `
    -Workspace C:\workspace\taskcontrol `
    -ResultPath C:\workspace\taskcontrol\task-baseline-control.json 2>&1 |
    ForEach-Object { Write-Output "TASK: $_" }
$taskExit = $LASTEXITCODE
Write-Output "TASK_CONTROL_EXIT=$taskExit"
if (Test-Path -LiteralPath 'C:\workspace\taskcontrol\task-baseline-control.json') {
    Get-Content -LiteralPath 'C:\workspace\taskcontrol\task-baseline-control.json' -Raw
}
exit $taskExit
