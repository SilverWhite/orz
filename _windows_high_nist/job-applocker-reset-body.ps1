$ErrorActionPreference = 'Continue'
Write-Output 'DIAG_START'
Write-Output '--- APPIDSVC BEFORE ---'
sc.exe query AppIDSvc 2>&1 | ForEach-Object { Write-Output "SVC: $_" }
Write-Output '--- SRP KEYS BEFORE ---'
foreach ($key in @('HKLM\SOFTWARE\Policies\Microsoft\Windows\SrpV2',
                   'HKLM\SOFTWARE\Policies\Microsoft\Windows\Safer')) {
    reg.exe query $key 2>&1 | ForEach-Object { Write-Output "REG: $key => $_" }
}
Write-Output '--- EFFECTIVE POLICY BEFORE ---'
try {
    $x = Get-AppLockerPolicy -Effective -Xml -ErrorAction Stop
    if ($x) { Write-Output ("POLICY_BEFORE_LEN=" + $x.Length) } else { Write-Output 'POLICY_BEFORE=EMPTY' }
}
catch { Write-Output ("POLICY_BEFORE_ERR=" + $_.Exception.Message) }
Write-Output '--- CLEANUP ---'
foreach ($key in @('HKLM:\SOFTWARE\Policies\Microsoft\Windows\SrpV2',
                   'HKLM:\SOFTWARE\Policies\Microsoft\Windows\Safer')) {
    if (Test-Path -LiteralPath $key) {
        try {
            Remove-Item -LiteralPath $key -Recurse -Force -ErrorAction Stop
            Write-Output "REMOVED $key"
        }
        catch { Write-Output ("REMOVE_ERR $key : " + $_.Exception.Message) }
    }
    else { Write-Output "ABSENT $key" }
}
Write-Output '--- GROUP POLICY STORE CLEANUP ---'
foreach ($dir in @('C:\Windows\System32\GroupPolicy\Machine\Microsoft\Windows\SrpV2',
                   'C:\Windows\System32\GroupPolicy\Machine\Microsoft\Windows\Safer')) {
    if (Test-Path -LiteralPath $dir) {
        try {
            Remove-Item -LiteralPath $dir -Recurse -Force -ErrorAction Stop
            Write-Output "GP_REMOVED $dir"
        }
        catch { Write-Output ("GP_REMOVE_ERR $dir : " + $_.Exception.Message) }
    }
    else { Write-Output "GP_ABSENT $dir" }
}
Write-Output '--- REGISTRY.POL SCAN ---'
$pol = 'C:\Windows\System32\GroupPolicy\Machine\registry.pol'
if (Test-Path -LiteralPath $pol) {
    $polBytes = [System.IO.File]::ReadAllBytes($pol)
    $polText = [System.Text.Encoding]::ASCII.GetString($polBytes)
    $hasSrp = $polText.Contains('SrpV2')
    $hasSafer = $polText.Contains('Safer')
    Write-Output "REGISTRY_POL_EXISTS=1 SIZE=$($polBytes.Length) HAS_SRPV2=$hasSrp HAS_SAFER=$hasSafer"
    if ($hasSrp -or $hasSafer) {
        $backup = $pol + '.bak-applocker-' + (Get-Date -Format 'yyyyMMddHHmmss')
        Copy-Item -LiteralPath $pol -Destination $backup -Force
        Write-Output "POL_BACKUP=$backup"
        Remove-Item -LiteralPath $pol -Force -ErrorAction Stop
        Write-Output 'POL_REMOVED=1'
    }
    else {
        Write-Output 'POL_REMOVED=0 (no applocker/srp entries)'
    }
}
else {
    Write-Output 'REGISTRY_POL_ABSENT=1'
}
Write-Output '--- GPUPDATE /FORCE ---'
gpupdate.exe /target:computer /force 2>&1 | ForEach-Object { Write-Output "GPU: $_" }
Write-Output '--- REG KEYS AFTER GPUPDATE ---'
foreach ($key in @('HKLM:\SOFTWARE\Policies\Microsoft\Windows\SrpV2',
                   'HKLM:\SOFTWARE\Policies\Microsoft\Windows\Safer')) {
    if (Test-Path -LiteralPath $key) {
        try {
            Remove-Item -LiteralPath $key -Recurse -Force -ErrorAction Stop
            Write-Output "REG_AFTER_REMOVED $key"
        }
        catch { Write-Output ("REG_AFTER_REMOVE_ERR $key : " + $_.Exception.Message) }
    }
    else { Write-Output "REG_AFTER_ABSENT $key" }
}
Write-Output '--- EFFECTIVE POLICY AFTER ---'
try {
    $y = Get-AppLockerPolicy -Effective -Xml -ErrorAction Stop
    if ($y -and $y.Length -gt 40) { Write-Output ("POLICY_AFTER_LEN=" + $y.Length) }
    else { Write-Output 'POLICY_AFTER=EMPTY' }
}
catch { Write-Output ("POLICY_AFTER_ERR=" + $_.Exception.Message) }
Write-Output '--- CODE INTEGRITY / SAC STATE ---'
$ciKey = 'HKLM:\SYSTEM\CurrentControlSet\Control\CI\Policy'
if (Test-Path -LiteralPath $ciKey) {
    Get-ItemProperty -LiteralPath $ciKey |
        Select-Object -Property * -ExcludeProperty PS* |
        ForEach-Object { $_.PSObject.Properties | ForEach-Object { Write-Output ("CI_REG: " + $_.Name + "=" + $_.Value) } }
}
else { Write-Output 'CI_KEY_ABSENT' }
Write-Output '--- SAC DISABLE ATTEMPT ---'
try {
    Set-ItemProperty -LiteralPath $ciKey -Name 'VerifiedAndReputablePolicyState' -Value 0 -Type DWord -ErrorAction Stop
    $sacAfter = (Get-ItemProperty -LiteralPath $ciKey -Name 'VerifiedAndReputablePolicyState' -ErrorAction Stop).VerifiedAndReputablePolicyState
    Write-Output "SAC_STATE_AFTER_SET=$sacAfter"
}
catch { Write-Output ("SAC_SET_ERR=" + $_.Exception.Message) }
foreach ($f in @('C:\Windows\System32\CodeIntegrity\SiPolicy.p7b',
                 'C:\Windows\System32\CodeIntegrity\CIPolicies\Active\SiPolicy.p7b')) {
    Write-Output ("CI_FILE $f EXISTS=" + (Test-Path -LiteralPath $f))
}
Write-Output '--- BLOCK EVENTS (last 30 min) ---'
try {
    Get-WinEvent -FilterHashtable @{ LogName = 'Microsoft-Windows-CodeIntegrity/Operational'; StartTime = (Get-Date).AddMinutes(-30) } -MaxEvents 8 -ErrorAction Stop |
        ForEach-Object { Write-Output ("CI_EV " + $_.TimeCreated.ToString('HH:mm:ss') + " id=" + $_.Id + " " + ($_.Message -replace "`r?`n", ' ')) }
}
catch { Write-Output ('CI_EV_ERR=' + $_.Exception.Message) }
try {
    Get-WinEvent -FilterHashtable @{ LogName = 'Microsoft-Windows-AppLocker/EXE and DLL'; StartTime = (Get-Date).AddMinutes(-30) } -MaxEvents 8 -ErrorAction Stop |
        ForEach-Object { Write-Output ("AL_EV " + $_.TimeCreated.ToString('HH:mm:ss') + " id=" + $_.Id + " " + ($_.Message -replace "`r?`n", ' ')) }
}
catch { Write-Output ('AL_EV_ERR=' + $_.Exception.Message) }
Write-Output 'NOTE: AppIDSvc in-memory policy cleared by the follow-up VM reboot.'
Write-Output 'DIAG_DONE'
exit 0
