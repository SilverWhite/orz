<#
    S4 elevated launcher: runs a target ps1 as Administrator via UAC.
    The target script is responsible for writing its own output file.
    Usage: powershell -NoProfile -ExecutionPolicy Bypass -File scripts\s4_run_elevated.ps1 -Target D:\CLI\scripts\s4_vm_inspect.ps1
    ASCII only.
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$Target,
    [string]$LogDir = 'D:\CLI\_windows_high_nist',
    [int]$TimeoutMinutes = 5
)

$ErrorActionPreference = 'Stop'
if (-not (Test-Path -LiteralPath $Target)) {
    throw "Target not found: $Target"
}

$stamp = Get-Date -Format 'yyyyMMdd_HHmmss'
$name = [System.IO.Path]::GetFileNameWithoutExtension($Target)
$exitFile = Join-Path $LogDir ("{0}_exit_{1}.txt" -f $name, $stamp)

$p = Start-Process -FilePath 'powershell.exe' -Verb RunAs `
    -ArgumentList @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', ('"{0}"' -f $Target)) `
    -PassThru

$deadline = (Get-Date).AddMinutes($TimeoutMinutes)
while (-not $p.HasExited -and (Get-Date) -lt $deadline) {
    Start-Sleep -Seconds 2
}
if ($p.HasExited) {
    ('EXIT=' + $p.ExitCode) | Out-File -Encoding utf8 $exitFile
} else {
    'TIMEOUT' | Out-File -Encoding utf8 $exitFile
    Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
}
Write-Output "exitFile=$exitFile"
