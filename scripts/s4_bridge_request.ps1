<#
    S4 admin bridge client (runs WITHOUT elevation, from the Codex sandbox).

      powershell -NoProfile -ExecutionPolicy Bypass -File D:\CLI\scripts\s4_bridge_request.ps1 `
          -Op stage -Stage sync -TimeoutSeconds 3600

    Writes a token-signed request JSON, waits for the worker's result file,
    prints it, and exits 0 only on STATUS=OK.
    ASCII only.
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidateSet('ping', 'vm-state', 'vm-start', 'vm-connect', 'vm-copy', 'vm-revert', 'vm-applocker-reset', 'vm-install-clash', 'vm-checkpoint', 'vm-hive-diag', 'vm-read-wrapup', 'vm-cred-lm', 'vm-cred-inject', 'vm-sync-orz', 'vm-agent', 'vm-diag-orz', 'vm-diag-signer', 'vm-acaf-reprovision', 'vm-env-probe', 'vm-env-provision', 'vm-wf12-probe', 'check-setup', 'stage', 'restart', 'quit')]
    [string]$Op,
    [ValidateSet('restore', 'sync', 'setup', 'control', 'nonadmin', 'highnist', 'taskcontrol', 'tasknonadmin', 'taskhighnist', 'agentcontrol', 'agenthighnist', 'netcheck', 'wrapup', 'all', 'full', 'reboot')]
    [string]$Stage = 'all',
    [ValidateSet('control', 'high-nist')]
    [string]$Arm = 'high-nist',
    [ValidateSet('tb2.1', 'friction')]
    [string]$TaskSet = 'tb2.1',
    [string]$TaskIds = '',
    [string]$RunTag = '',
    [switch]$DryRun,
    [int]$TimeoutSeconds = 5400,
    [string]$Src = '',
    [string]$Dst = '',
    [string]$KeyFile = '',
    [string]$CheckpointName = '',
    [string]$BridgeRoot = 'D:\CLI\_windows_high_nist\bridge'
)

$ErrorActionPreference = 'Stop'
$reqDir = Join-Path $BridgeRoot 'requests'
$resDir = Join-Path $BridgeRoot 'results'
$pidFile = Join-Path $BridgeRoot 'worker.pid'
$tokenFile = Join-Path $BridgeRoot 'token.txt'

if (-not (Test-Path -LiteralPath $tokenFile)) {
    Write-Output 'ERROR=NO_TOKEN_FILE (start the bridge first)'
    exit 1
}
if (-not (Test-Path -LiteralPath $reqDir)) {
    New-Item -ItemType Directory -Path $reqDir -Force | Out-Null
}
if (-not (Test-Path -LiteralPath $resDir)) {
    New-Item -ItemType Directory -Path $resDir -Force | Out-Null
}

$token = (Get-Content -LiteralPath $tokenFile -Raw).Trim()
$id = [guid]::NewGuid().ToString('N')
$body = [ordered]@{ id = $id; token = $token; op = $Op }
if ($Op -eq 'stage') {
    $body.stage = $Stage
    if ($CheckpointName) {
        $ckAllowed = @('S4-BASE-INSTALLED', 'S4-BASE-NET-2026-09-02')
        if ($ckAllowed -notcontains $CheckpointName) {
            Write-Output "ERROR=BAD_CHECKPOINT $CheckpointName"
            exit 1
        }
        $body.checkpoint = $CheckpointName
    }
}
if ($Op -eq 'vm-copy') {
    $body.src = $Src
    $body.dst = $Dst
}
if ($Op -eq 'vm-checkpoint') {
    $body.checkpoint = $CheckpointName
}
if ($Op -eq 'vm-agent') {
    $body.arm = $Arm
    $body.taskSet = $TaskSet
    $body.taskIds = $TaskIds
    if ($KeyFile) {
        $body.keyFile = $KeyFile
    }
    if ($RunTag) {
        $body.runTag = $RunTag
    }
    if ($DryRun) {
        $body.dryRun = $true
    }
}
$json = $body | ConvertTo-Json -Compress

$tmpReq = Join-Path $reqDir ("request-$id.json.tmp")
$reqPath = Join-Path $reqDir ("request-$id.json")
[System.IO.File]::WriteAllText($tmpReq, $json, (New-Object System.Text.UTF8Encoding($false)))
Move-Item -LiteralPath $tmpReq -Destination $reqPath -Force
Write-Output "REQUESTED=1 OP=$Op ID=$id"

$resultPath = Join-Path $resDir ("result-$id.txt")
$deadline = (Get-Date).AddSeconds($TimeoutSeconds)
$lastPidCheck = [DateTime]::MinValue

while ((Get-Date) -lt $deadline) {
    if (Test-Path -LiteralPath $resultPath) {
        $content = @(Get-Content -LiteralPath $resultPath -Encoding UTF8)
        $content
        $statusLine = $content | Where-Object { $_ -match '^STATUS=(OK|FAIL)$' } | Select-Object -Last 1
        if ($statusLine -match '^STATUS=OK$') {
            exit 0
        }
        exit 1
    }
    if ((Get-Date) -gt $lastPidCheck.AddSeconds(10)) {
        $lastPidCheck = Get-Date
        if (Test-Path -LiteralPath $pidFile) {
            try {
                $workerPid = [int]((Get-Content -LiteralPath $pidFile -Raw).Trim())
                $alive = Get-Process -Id $workerPid -ErrorAction SilentlyContinue
                if (-not $alive) {
                    Write-Output 'ERROR=BRIDGE_WORKER_NOT_RUNNING'
                    exit 1
                }
            }
            catch { }
        }
    }
    Start-Sleep -Seconds 3
}

Write-Output "ERROR=TIMEOUT ID=$id"
exit 1
