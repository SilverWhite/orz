$pw = 'S4Run!2026'
& net.exe user AgentUser $pw 2>&1 | ForEach-Object { Write-Output "NET: $_" }
Write-Output "NET_EXIT=$LASTEXITCODE"
[Environment]::SetEnvironmentVariable('ORZ_WINDOWS_RUN_USER', 'AgentUser', 'Machine')
[Environment]::SetEnvironmentVariable('ORZ_WINDOWS_RUN_USER_PASSWORD', $pw, 'Machine')
[Environment]::SetEnvironmentVariable('ORZ_WINDOWS_RUN_USER_DOMAIN', 'DESKTOP-QMAPMFH', 'Machine')
Write-Output "ENV_USER=$([Environment]::GetEnvironmentVariable('ORZ_WINDOWS_RUN_USER','Machine'))"
Write-Output "ENV_DOMAIN=$([Environment]::GetEnvironmentVariable('ORZ_WINDOWS_RUN_USER_DOMAIN','Machine'))"
