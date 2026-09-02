<#
    S4 VM prep phase 2b (host-side elevated):
      A) clean stale machine PATH entries
      B) re-take baseline snapshot (python + orz in place)
      C) control-arm enforcement probe (pre-hardening baseline)
    ASCII only.
#>
[CmdletBinding()]
param(
    [string]$OutFile = 'D:\CLI\_windows_high_nist\vm-prep-phase2b-result.txt'
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
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $runner `
        -JobBodyFile $bodyFile -OutFile $res -TimeoutSeconds $TimeoutSeconds
    if ($LASTEXITCODE -ne 0) { throw "guest job $Name runner failed (exit $LASTEXITCODE)" }
    $lines.Add("===== JOB $Name =====")
    foreach ($l in (Get-Content -LiteralPath $res -Encoding UTF8)) { $lines.Add($l) }
}

try {
    $bodyA = @'
$p = [Environment]::GetEnvironmentVariable('Path','Machine')
$entries = $p -split ';' | ForEach-Object {
    $e = $_.TrimEnd('\')
    if ($e -and $e -ne 'C:\Program' -and $e -ne 'C:\Program\Scripts') { $_ }
}
$new = ($entries -join ';')
[Environment]::SetEnvironmentVariable('Path', $new, 'Machine')
Write-Output "PATH_FINAL=$([Environment]::GetEnvironmentVariable('Path','Machine'))"
'@
    Invoke-GuestJob -Name 'clean-path' -Body $bodyA

    try {
        $snap = Get-VMSnapshot -VMName 'win-s4' -Name 'S4-BASE-INSTALLED' -ErrorAction SilentlyContinue
        if ($snap) {
            Remove-VMSnapshot -VMName 'win-s4' -Name 'S4-BASE-INSTALLED' -Confirm:$false
            $lines.Add('===== OLD SNAPSHOT REMOVED =====')
        }
        Checkpoint-VM -Name 'win-s4' -SnapshotName 'S4-BASE-INSTALLED' -Confirm:$false -ErrorAction Stop
        $lines.Add('===== SNAPSHOT S4-BASE-INSTALLED (v2) OK =====')
    } catch {
        $lines.Add("SNAPSHOT_ERR: $($_.Exception.Message)")
    }

    $bodyC = @'
$env:Path = [Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [Environment]::GetEnvironmentVariable('Path','User')
New-Item -ItemType Directory -Path 'C:\s4\results' -Force | Out-Null
& C:\s4\run\run_enforcement_probe.ps1 -Arm control -ResultPath C:\s4\results\enforcement-probe-control.json
exit $LASTEXITCODE
'@
    Invoke-GuestJob -Name 'probe-control' -Body $bodyC -TimeoutSeconds 300
}
catch {
    $lines.Add("ERROR: $($_.Exception.Message)")
}
$lines | Set-Content -LiteralPath $OutFile -Encoding utf8
