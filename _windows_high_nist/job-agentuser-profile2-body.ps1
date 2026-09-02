$out = 'C:\s4\tools\agentuser-first.txt'
Remove-Item -LiteralPath $out -Force -ErrorAction SilentlyContinue
& schtasks.exe /create /f /tn s4agentuser /tr "cmd.exe /c echo ok > C:\s4\tools\agentuser-first.txt" /sc once /st 00:00 /ru AgentUser /rp S4Run!2026 2>&1 | ForEach-Object { Write-Output "SCH: $_" }
& schtasks.exe /run /tn s4agentuser 2>&1 | ForEach-Object { Write-Output "RUN: $_" }
$deadline = (Get-Date).AddMinutes(2)
while (-not (Test-Path -LiteralPath $out) -and (Get-Date) -lt $deadline) { Start-Sleep -Seconds 2 }
$nt = 'C:\Users\AgentUser\NTUSER.DAT'
Write-Output "AGENTUSER_NTUSER_EXISTS=$(Test-Path -LiteralPath $nt)"
if (Test-Path -LiteralPath $out) { Get-Content -LiteralPath $out | ForEach-Object { Write-Output "FIRST_RUN: $_" } }
& schtasks.exe /delete /f /tn s4agentuser 2>&1 | Out-Null
