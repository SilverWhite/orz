$env:Path = [Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [Environment]::GetEnvironmentVariable('Path','User')
New-Item -ItemType Directory -Path 'C:\s4\results' -Force | Out-Null
& C:\s4\_windows_high_nist\hardening\apply_hardening.ps1 -Arm high-nist -RunUser AgentUser -WorkspaceRoot C:\workspace -DeepSeekIp '221.204.163.76' -EnableAppLocker
exit $LASTEXITCODE
