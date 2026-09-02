<#
    S4 VM enforcement probes retry (host-side elevated):
      A) sync fixed windows_sandbox.py into the VM
      B) non-admin probe
      C) high-nist probe
    ASCII only.
#>
[CmdletBinding()]
param(
    [string]$OutFile = 'D:\CLI\_windows_high_nist\vm-probe-arms2-result.txt'
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

    $bodyA = @'
$env:Path = [Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [Environment]::GetEnvironmentVariable('Path','User')
New-Item -ItemType Directory -Path 'C:\s4\results' -Force | Out-Null
& C:\s4\_windows_high_nist\run\run_enforcement_probe.ps1 -Arm non-admin -ResultPath C:\s4\results\enforcement-probe-non-admin.json
exit $LASTEXITCODE
'@
    Invoke-GuestJob -Name 'probe-non-admin' -Body $bodyA -TimeoutSeconds 300

    $bodyB = @'
$env:Path = [Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [Environment]::GetEnvironmentVariable('Path','User')
New-Item -ItemType Directory -Path 'C:\s4\results' -Force | Out-Null
& C:\s4\_windows_high_nist\run\run_enforcement_probe.ps1 -Arm high-nist -ResultPath C:\s4\results\enforcement-probe-high-nist.json
exit $LASTEXITCODE
'@
    Invoke-GuestJob -Name 'probe-high-nist' -Body $bodyB -TimeoutSeconds 300
}
catch {
    $lines.Add("ERROR: $($_.Exception.Message)")
}
$lines | Set-Content -LiteralPath $OutFile -Encoding utf8
