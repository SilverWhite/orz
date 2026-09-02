<#
    S4 VM hardening driver (host-side elevated):
      A) re-take baseline snapshot (full harness present)
      B) resolve api.deepseek.com A records inside the VM
      C) run apply_hardening.ps1 -Arm high-nist -EnableAppLocker
    ASCII only.
#>
[CmdletBinding()]
param(
    [string]$OutFile = 'D:\CLI\_windows_high_nist\vm-harden-result.txt'
)

$ErrorActionPreference = 'Stop'
$lines = New-Object System.Collections.Generic.List[string]
$runner = 'D:\CLI\scripts\s4_vm_run_elev.ps1'

function Invoke-GuestJob {
    param([string]$Name, [string]$Body, [int]$TimeoutSeconds = 1200)
    $res = Join-Path 'D:\CLI\_windows_high_nist' ("job-$Name-result.txt")
    $bodyFile = Join-Path 'D:\CLI\_windows_high_nist' ("job-$Name-body.ps1")
    Set-Content -LiteralPath $bodyFile -Value $Body -Encoding utf8
    Remove-Item -LiteralPath $res -Force -ErrorAction SilentlyContinue
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $runner `
        -JobBodyFile $bodyFile -OutFile $res -TimeoutSeconds $TimeoutSeconds
    if ($LASTEXITCODE -ne 0) { throw "guest job $Name runner failed (exit $LASTEXITCODE)" }
    $lines.Add("===== JOB $Name =====")
    foreach ($l in (Get-Content -LiteralPath $res -Encoding UTF8)) { $lines.Add($l) }
}

try {
    # A) baseline snapshot with the full harness
    try {
        $snap = Get-VMSnapshot -VMName 'win-s4' -Name 'S4-BASE-INSTALLED' -ErrorAction SilentlyContinue
        if ($snap) {
            Remove-VMSnapshot -VMName 'win-s4' -Name 'S4-BASE-INSTALLED' -Confirm:$false
        }
        Checkpoint-VM -Name 'win-s4' -SnapshotName 'S4-BASE-INSTALLED' -Confirm:$false -ErrorAction Stop
        $lines.Add('===== SNAPSHOT S4-BASE-INSTALLED (full harness) OK =====')
    } catch {
        $lines.Add("SNAPSHOT_ERR: $($_.Exception.Message)")
    }

    # B) resolve DeepSeek A records in the VM
    $bodyB = @'
$ips = Resolve-DnsName api.deepseek.com -Type A -ErrorAction SilentlyContinue |
    Where-Object { $_.Type -eq 'A' } | Select-Object -ExpandProperty IPAddress -Unique
Write-Output "IPS=$($ips -join ',')"
'@
    $resB = Join-Path 'D:\CLI\_windows_high_nist' 'job-resolve-deepseek-result.txt'
    $bodyFileB = Join-Path 'D:\CLI\_windows_high_nist' 'job-resolve-deepseek-body.ps1'
    Set-Content -LiteralPath $bodyFileB -Value $bodyB -Encoding utf8
    Remove-Item -LiteralPath $resB -Force -ErrorAction SilentlyContinue
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $runner `
        -JobBodyFile $bodyFileB -OutFile $resB -TimeoutSeconds 120
    if ($LASTEXITCODE -ne 0) { throw "resolve job failed" }
    $deepseekIp = ''
    foreach ($l in (Get-Content -LiteralPath $resB -Encoding UTF8)) {
        $lines.Add($l)
        if ($l -match '^IPS=(.+)$') {
            $deepseekIp = ($Matches[1] -split ',')[0].Trim()
        }
    }
    if (-not $deepseekIp) {
        $lines.Add('DEEPSEEK_IP_RESOLVE_FAILED')
        throw 'could not resolve api.deepseek.com'
    }
    $lines.Add("DEEPSEEK_IP=$deepseekIp")

    # C) hardening high-nist
    $bodyC = @"
`$env:Path = [Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [Environment]::GetEnvironmentVariable('Path','User')
New-Item -ItemType Directory -Path 'C:\s4\results' -Force | Out-Null
& C:\s4\_windows_high_nist\hardening\apply_hardening.ps1 -Arm high-nist -RunUser AgentUser -WorkspaceRoot C:\workspace -DeepSeekIp '$deepseekIp' -EnableAppLocker
exit `$LASTEXITCODE
"@
    Invoke-GuestJob -Name 'harden-high-nist' -Body $bodyC -TimeoutSeconds 1200
}
catch {
    $lines.Add("ERROR: $($_.Exception.Message)")
}
$lines | Set-Content -LiteralPath $OutFile -Encoding utf8
