$out = 'C:\s4\tools\agentuser-first.txt'
Remove-Item -LiteralPath $out -Force -ErrorAction SilentlyContinue
& schtasks.exe /create /f /tn s4agentuser /tr "cmd.exe /c echo ok > C:\s4\tools\agentuser-first.txt" /sc once /st 00:00 /ru AgentUser /rp S4Run!2026 2>&1 | ForEach-Object { Write-Output "SCH: $_" }
Write-Output "CREATE_EXIT=$LASTEXITCODE"
& schtasks.exe /run /tn s4agentuser 2>&1 | ForEach-Object { Write-Output "RUN: $_" }
Write-Output "RUN_EXIT=$LASTEXITCODE"
$deadline = (Get-Date).AddMinutes(2)
while (-not (Test-Path -LiteralPath $out) -and (Get-Date) -lt $deadline) { Start-Sleep -Seconds 2 }
$nt = 'C:\Users\AgentUser\NTUSER.DAT'
Write-Output "AGENTUSER_NTUSER_EXISTS=$(Test-Path -LiteralPath $nt)"
Write-Output "PROFILE_DIR_EXISTS=$(Test-Path -LiteralPath 'C:\Users\AgentUser')"
& schtasks.exe /delete /f /tn s4agentuser 2>&1 | Out-Null
