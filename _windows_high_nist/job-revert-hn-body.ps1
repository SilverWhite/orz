$env:Path = [Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [Environment]::GetEnvironmentVariable('Path','User')
$hard = 'C:\s4\_windows_high_nist\hardening\apply_hardening.ps1'
Write-Output '===== REVERT HIGH-NIST ====='
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File $hard -Arm high-nist -RunUser AgentUser -WorkspaceRoot C:\workspace -EnableAppLocker -Revert 2>&1 |
    ForEach-Object { Write-Output "RVH: $_" }
$code = $LASTEXITCODE
Write-Output "REVERT_HN_EXIT=$code"
exit $code
