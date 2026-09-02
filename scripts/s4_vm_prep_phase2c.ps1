<#
    S4 VM prep phase 2c (host-side elevated):
      A) relocate harness dirs to mirror repo layout
         C:\s4\_windows_high_nist\{run,policy,hardening}
      B) re-run control-arm enforcement probe
    ASCII only.
#>
[CmdletBinding()]
param(
    [string]$OutFile = 'D:\CLI\_windows_high_nist\vm-prep-phase2c-result.txt'
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
$base = 'C:\s4\_windows_high_nist'
New-Item -ItemType Directory -Path (Join-Path $base 'run') -Force | Out-Null
New-Item -ItemType Directory -Path (Join-Path $base 'policy') -Force | Out-Null
New-Item -ItemType Directory -Path (Join-Path $base 'hardening') -Force | Out-Null
foreach ($d in @('run','policy','hardening')) {
    $src = Join-Path 'C:\s4' $d
    foreach ($f in (Get-ChildItem -LiteralPath $src -File)) {
        $dst = Join-Path (Join-Path $base $d) $f.Name
        Move-Item -LiteralPath $f.FullName -Destination $dst -Force
        Write-Output "MOVED $($f.Name) -> $dst"
    }
}
'@
    Invoke-GuestJob -Name 'relocate-harness' -Body $bodyA

    $bodyB = @'
$env:Path = [Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [Environment]::GetEnvironmentVariable('Path','User')
New-Item -ItemType Directory -Path 'C:\s4\results' -Force | Out-Null
& C:\s4\_windows_high_nist\run\run_enforcement_probe.ps1 -Arm control -ResultPath C:\s4\results\enforcement-probe-control.json
exit $LASTEXITCODE
'@
    Invoke-GuestJob -Name 'probe-control' -Body $bodyB -TimeoutSeconds 300
}
catch {
    $lines.Add("ERROR: $($_.Exception.Message)")
}
$lines | Set-Content -LiteralPath $OutFile -Encoding utf8
