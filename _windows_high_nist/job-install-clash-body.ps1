$ErrorActionPreference = 'Continue'
$exe = 'C:\Users\HL\Downloads\Clash.Verge_2.5.2_x64-setup.exe'
$bat = 'C:\s4\tools\s4-clash-install.bat'
$marker = 'C:\Windows\Temp\s4-clash-install-rc.txt'
$task = 's4clashinstall'
Remove-Item -LiteralPath $marker -Force -ErrorAction SilentlyContinue
@(
  '@echo off',
  'REM silent install',
  '"C:\Users\HL\Downloads\Clash.Verge_2.5.2_x64-setup.exe" /S',
  'set RC=%errorlevel%',
  'echo RC=%RC% > C:\Windows\Temp\s4-clash-install-rc.txt',
  'exit /b %RC%'
) | Set-Content -LiteralPath $bat -Encoding ascii
Write-Output "EXE_EXISTS=$(Test-Path -LiteralPath $exe)"
schtasks.exe /delete /tn $task /f 2>&1 | Out-Null
$tr = 'cmd.exe /c C:\s4\tools\s4-clash-install.bat'
schtasks.exe /create /f /tn $task /tr $tr /sc once /st 23:59 /ru HL /rp 123456 /rl highest 2>&1 | ForEach-Object { Write-Output "TASK: $_" }
if ($LASTEXITCODE -ne 0) { Write-Output 'TASK_CREATE_FAIL'; exit 1 }
schtasks.exe /run /tn $task 2>&1 | ForEach-Object { Write-Output "RUN: $_" }
$deadline = (Get-Date).AddSeconds(300)
$done = $false
$rcText = ''
while ((Get-Date) -lt $deadline) {
    if (Test-Path -LiteralPath $marker) { $done = $true; break }
    Start-Sleep -Seconds 3
}
if ($done) {
    $readDeadline = (Get-Date).AddSeconds(30)
    while ((Get-Date) -lt $readDeadline -and -not ($rcText -match 'RC=')) {
        try {
            $rcText = (Get-Content -LiteralPath $marker -Raw -ErrorAction Stop).Trim()
        }
        catch { $rcText = '' }
        if (-not ($rcText -match 'RC=')) {
            Start-Sleep -Seconds 1
        }
    }
    Write-Output "MARK: $rcText"
    if ($rcText -match 'RC=0') { Write-Output 'CLASH_INSTALL_OK=1' } else { Write-Output 'CLASH_INSTALL_OK=0' }
}
else {
    Write-Output 'CLASH_INSTALL_TIMEOUT=1'
    schtasks.exe /end /tn $task 2>&1 | Out-Null
}
try {
    $taskInfo = Get-ScheduledTaskInfo -TaskName $task -ErrorAction Stop
    Write-Output "LAST_TASK_RESULT=$($taskInfo.LastTaskResult)"
}
catch { Write-Output "LAST_TASK_RESULT_ERR=$($_.Exception.Message)" }
schtasks.exe /delete /tn $task /f 2>&1 | Out-Null
if ($done -and ($rcText -match 'RC=0')) { exit 0 } else { exit 1 }
