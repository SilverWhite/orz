<#
    S4 admin bridge launcher (run from an ELEVATED PowerShell):

      powershell -NoProfile -ExecutionPolicy Bypass -File D:\CLI\scripts\s4_admin_bridge_start.ps1

    Generates token.txt on first run, starts the worker hidden, and prints
    the PID and paths.  Stop the worker later with op=quit through
    s4_bridge_request.ps1.
    ASCII only.
#>
[CmdletBinding()]
param(
    [string]$BridgeRoot = 'D:\CLI\_windows_high_nist\bridge'
)

$ErrorActionPreference = 'Stop'
$ident = [System.Security.Principal.WindowsIdentity]::GetCurrent()
$princ = [System.Security.Principal.WindowsPrincipal]::new($ident)
if (-not $princ.IsInRole([System.Security.Principal.WindowsBuiltInRole]::Administrator)) {
    Write-Output 'ERROR=NOT_ADMIN (open an elevated PowerShell first)'
    exit 1
}

$reqDir = Join-Path $BridgeRoot 'requests'
$resDir = Join-Path $BridgeRoot 'results'
$tokenFile = Join-Path $BridgeRoot 'token.txt'
$pidFile = Join-Path $BridgeRoot 'worker.pid'
$outLog = Join-Path $BridgeRoot 'worker-console.log'
$errLog = Join-Path $BridgeRoot 'worker-console.err.log'

foreach ($d in @($BridgeRoot, $reqDir, $resDir)) {
    if (-not (Test-Path -LiteralPath $d)) {
        New-Item -ItemType Directory -Path $d -Force | Out-Null
    }
}

if (-not (Test-Path -LiteralPath $tokenFile)) {
    $bytes = New-Object byte[] 32
    $rng = [System.Security.Cryptography.RandomNumberGenerator]::Create()
    $rng.GetBytes($bytes)
    $token = -join ($bytes | ForEach-Object { $_.ToString('x2') })
    [System.IO.File]::WriteAllText($tokenFile, $token, (New-Object System.Text.ASCIIEncoding))
    Write-Output 'TOKEN_CREATED=1'
}

$workerPid = -1
if (Test-Path -LiteralPath $pidFile) {
    try { $workerPid = [int]((Get-Content -LiteralPath $pidFile -Raw).Trim()) } catch { $workerPid = -1 }
    $alive = $null
    if ($workerPid -gt 0) {
        $alive = Get-Process -Id $workerPid -ErrorAction SilentlyContinue
    }
    if ($alive) {
        Write-Output "BRIDGE_ALREADY_RUNNING=1"
        Write-Output "PID=$workerPid"
        Write-Output "TOKEN_FILE=$tokenFile"
        Write-Output "REQUESTS=$reqDir"
        Write-Output "RESULTS=$resDir"
        exit 0
    }
}

$worker = Join-Path $PSScriptRoot 's4_admin_bridge.ps1'
$p = Start-Process -FilePath 'powershell.exe' `
    -ArgumentList @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', $worker, '-BridgeRoot', $BridgeRoot) `
    -WindowStyle Hidden -PassThru `
    -RedirectStandardOutput $outLog -RedirectStandardError $errLog
[System.IO.File]::WriteAllText($pidFile, [string]$p.Id, (New-Object System.Text.ASCIIEncoding))

Write-Output 'BRIDGE_STARTED=1'
Write-Output "PID=$($p.Id)"
Write-Output "TOKEN_FILE=$tokenFile"
Write-Output "REQUESTS=$reqDir"
Write-Output "RESULTS=$resDir"
Write-Output "WORKER_LOG=$(Join-Path $BridgeRoot 'worker.log')"
Write-Output "CONSOLE_LOG=$outLog"
