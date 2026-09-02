$env:Path = [Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [Environment]::GetEnvironmentVariable('Path','User')
$env:PYTHONPATH = 'C:\s4'
$ErrorActionPreference = 'Continue'
$root = 'C:\workspace\wrapup'
$py = 'C:\Program Files\Python312\python.exe'
$hard = 'C:\s4\_windows_high_nist\hardening\apply_hardening.ps1'
$probeRun = 'C:\s4\_windows_high_nist\run\run_enforcement_probe.ps1'
$sandboxCli = 'C:\s4\scripts\run_windows_native_sandbox_command.py'
New-Item -ItemType Directory -Path $root -Force | Out-Null
$summary = [ordered]@{
    ran_at = (Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ')
    apply_exit = -1
    applocker_effective = $false
    applocker_srpv2 = $false
    applocker_exe_rules = 0
    probe1 = $null
    probe2 = $null
    hive_5023_1 = $false
    hive_5023_2 = $false
    hive_mounted_pre_apply = $false
    cred_nonadmin = $null
    cred_highnist = $null
    hive_mounted_post = $false
    hive_locked_post = $false
    errors = New-Object System.Collections.ArrayList
}
# Pre-apply guard: the AgentUser hive must not already be mounted
$u = [ADSI]"WinNT://$env:COMPUTERNAME/AgentUser"
$sidBytes = $u.objectSID[0]
$sid = [System.Security.Principal.SecurityIdentifier]::new($sidBytes, 0).Value
$hkuOut = (& reg.exe query HKU 2>&1) -join "`n"
$summary.hive_mounted_pre_apply = $hkuOut.Contains($sid)
Write-Output "HIVE_PRE_APPLY mounted=$($summary.hive_mounted_pre_apply)"
if ($summary.hive_mounted_pre_apply) {
    [void]$summary.errors.Add('AgentUser hive mounted before apply (leaked profile load)')
}
Write-Output '===== WRAPUP APPLY HIGH-NIST ====='
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File $hard -Arm high-nist -RunUser AgentUser -WorkspaceRoot C:\workspace -DeepSeekIp 221.204.163.76 -EnableAppLocker 2>&1 | ForEach-Object { Write-Output "WRAP_APPLY: $_" }
$summary.apply_exit = $LASTEXITCODE
Write-Output "WRAP_APPLY_EXIT=$($summary.apply_exit)"
if ($summary.apply_exit -ne 0) {
    [void]$summary.errors.Add('apply high-nist failed')
}
# AppLocker re-verify
$alXml = ''
try {
    $alXml = [string](Get-AppLockerPolicy -Effective -Xml -ErrorAction Stop)
}
catch {
    [void]$summary.errors.Add('Get-AppLockerPolicy failed: ' + $_.Exception.Message)
}
try {
    [xml]$x = $alXml
    $exeColl = @($x.AppLockerPolicy.RuleCollection | Where-Object { $_.Type -eq 'Exe' })
    if ($exeColl.Count -gt 0) {
        $summary.applocker_effective = $true
        $summary.applocker_enforcement = [string]$exeColl[0].EnforcementMode
        $summary.applocker_exe_rules = @($exeColl[0].ChildNodes | Where-Object { $_.NodeType -eq 'Element' }).Count
    }
}
catch {
    [void]$summary.errors.Add('AppLocker XML parse failed: ' + $_.Exception.Message)
}
& reg.exe query 'HKLM\SOFTWARE\Policies\Microsoft\Windows\SrpV2' 2>$null | Out-Null
$summary.applocker_srpv2 = ($LASTEXITCODE -eq 0)
Write-Output "APPLOCKER effective=$($summary.applocker_effective) srpv2=$($summary.applocker_srpv2) exe_rules=$($summary.applocker_exe_rules)"
if (-not $summary.applocker_effective -or -not $summary.applocker_srpv2) {
    [void]$summary.errors.Add('AppLocker verify failed')
}
$summary | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $root 'wrapup-apply.json') -Encoding utf8
Write-Output 'WRAPUP_APPLY_DONE'
if ($summary.errors.Count -gt 0) {
    Write-Output "WRAPUP_ERRORS=$($summary.errors.Count)"
    foreach ($e in $summary.errors) { Write-Output "WRAPUP_ERROR: $e" }
    exit 1
}
exit 0
