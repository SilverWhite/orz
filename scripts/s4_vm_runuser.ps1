<#
    S4 VM run-user mode setup (host-side elevated):
      A) sync windows_sandbox.py (run-user mode)
      B) set AgentUser password + machine env ORZ_WINDOWS_RUN_USER_*
      C) log AgentUser on once (profile + NTUSER.DAT)
      D) run high-nist enforcement probe
    ASCII only.
#>
[CmdletBinding()]
param(
    [string]$OutFile = 'D:\CLI\_windows_high_nist\vm-runuser-result.txt'
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
$pw = 'S4Run!2026'
& net.exe user AgentUser $pw 2>&1 | ForEach-Object { Write-Output "NET: $_" }
Write-Output "NET_EXIT=$LASTEXITCODE"
[Environment]::SetEnvironmentVariable('ORZ_WINDOWS_RUN_USER', 'AgentUser', 'Machine')
[Environment]::SetEnvironmentVariable('ORZ_WINDOWS_RUN_USER_PASSWORD', $pw, 'Machine')
[Environment]::SetEnvironmentVariable('ORZ_WINDOWS_RUN_USER_DOMAIN', 'DESKTOP-QMAPMFH', 'Machine')
Write-Output "ENV_USER=$([Environment]::GetEnvironmentVariable('ORZ_WINDOWS_RUN_USER','Machine'))"
Write-Output "ENV_DOMAIN=$([Environment]::GetEnvironmentVariable('ORZ_WINDOWS_RUN_USER_DOMAIN','Machine'))"
'@
    Invoke-GuestJob -Name 'runuser-setup' -Body $bodyB

    $bodyC = @'
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
'@
    Invoke-GuestJob -Name 'agentuser-profile' -Body $bodyC

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
