$cfg = 'C:\s4\tools\secpol.cfg'
$sdb = 'C:\s4\tools\secpol.sdb'
Remove-Item -LiteralPath $cfg,$sdb -Force -ErrorAction SilentlyContinue
& secedit.exe /export /cfg $cfg /areas USER_RIGHTS 2>&1 | ForEach-Object { Write-Output "EXPORT: $_" }
$user = [ADSI]"WinNT://$env:COMPUTERNAME/AgentUser"
$sidBytes = $user.objectSID[0]
$sid = [System.Security.Principal.SecurityIdentifier]::new($sidBytes, 0).Value
Write-Output "AGENTUSER_SID=$sid"
$cfgLines = Get-Content -LiteralPath $cfg
$outLines = New-Object System.Collections.Generic.List[string]
$patched = $false
foreach ($line in $cfgLines) {
    if ($line -match '^SeBatchLogonRight\s*=') {
        $outLines.Add($line.TrimEnd() + ",*$sid")
        $patched = $true
    } else {
        $outLines.Add($line)
    }
}
if (-not $patched) {
    $outLines.Add("SeBatchLogonRight = *$sid")
}
$outLines | Set-Content -LiteralPath $cfg -Encoding ascii
& secedit.exe /configure /db $sdb /cfg $cfg /areas USER_RIGHTS 2>&1 | ForEach-Object { Write-Output "CONFIGURE: $_" }
Write-Output "SECEDIT_EXIT=$LASTEXITCODE"
