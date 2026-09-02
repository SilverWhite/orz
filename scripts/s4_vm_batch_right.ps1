<#
    S4 VM grant SeBatchLogonRight to AgentUser + retry profile + probe
    (host-side elevated).  ASCII only.
#>
[CmdletBinding()]
param(
    [string]$OutFile = 'D:\CLI\_windows_high_nist\vm-batch-right-result.txt'
)

$ErrorActionPreference = 'Stop'
$lines = New-Object System.Collections.Generic.List[string]
$runner = 'D:\CLI\scripts\s4_vm_run_elev.ps1'

function Invoke-GuestJob {
    param([string]$Name, [string]$Body, [int]$TimeoutSeconds = 600)
    $res = Join-Path 'D:\CLI\_windows_high_nist' ("job-$Name-result.txt")
    $bodyFile = Join-Path 'D:\CLI\_windows_high_nist' ("job-$Name-body.ps1")
    Set-Content -LiteralPath $bodyFile -Value $Body -Encoding utf8
    Remove-Item -LiteralPath $res -Force -ErrorAction SilentlyContinue
    $oldEap = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $runner `
        -JobBodyFile $bodyFile -OutFile $res -TimeoutSeconds $TimeoutSeconds
    $code = $LASTEXITCODE
    $ErrorActionPreference = $oldEap
    if ($code -ne 0) { throw "guest job $Name runner failed (exit $code)" }
    $lines.Add("===== JOB $Name =====")
    foreach ($l in (Get-Content -LiteralPath $res -Encoding UTF8)) { $lines.Add($l) }
}

try {
    $sec = ConvertTo-SecureString '123456' -AsPlainText -Force
    $cred = New-Object System.Management.Automation.PSCredential('HL', $sec)
    $sess = New-PSSession -VMName 'win-s4' -Credential $cred
    Copy-Item -LiteralPath 'D:\CLI\assurance\windows_sandbox.py' `
        -Destination 'C:\s4\assurance\windows_sandbox.py' -ToSession $sess -Force
    $srcHash = (Get-FileHash -LiteralPath 'D:\CLI\assurance\windows_sandbox.py' -Algorithm SHA256).Hash
    $dstHash = Invoke-Command -Session $sess -ScriptBlock {
        (Get-FileHash -LiteralPath 'C:\s4\assurance\windows_sandbox.py' -Algorithm SHA256).Hash
    }
    $lines.Add("SYNC_WINDOWS_SANDBOX=$($srcHash -eq $dstHash)")
    Remove-PSSession -Session $sess -ErrorAction SilentlyContinue

    $bodyB = @'
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
'@
    Invoke-GuestJob -Name 'grant-batch-right' -Body $bodyB

    $bodyC = @'
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
'@
    Invoke-GuestJob -Name 'agentuser-profile2' -Body $bodyC

    $bodyD = @'
$env:Path = [Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [Environment]::GetEnvironmentVariable('Path','User')
New-Item -ItemType Directory -Path 'C:\s4\results' -Force | Out-Null
& C:\s4\_windows_high_nist\run\run_enforcement_probe.ps1 -Arm high-nist -ResultPath C:\s4\results\enforcement-probe-high-nist.json
exit $LASTEXITCODE
'@
    Invoke-GuestJob -Name 'probe-high-nist' -Body $bodyD -TimeoutSeconds 300
}
catch {
    $lines.Add("ERROR: $($_.Exception.Message)")
}
$lines | Set-Content -LiteralPath $OutFile -Encoding utf8
