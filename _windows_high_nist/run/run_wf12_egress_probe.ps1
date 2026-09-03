<#
TER T2.2 (W-F12) egress baseline / refusal-layer probe.

Measures how long each of the four client shapes takes to FAIL against a
walled target inside the high-nist guest:
  - Test-NetConnection (TCP 443)
  - curl.exe
  - Invoke-WebRequest
  - python requests

Output: JSON rows {client, target, elapsed_ms, status} + wall-clock total.
Run inside the wall (VM high-nist) BEFORE (baseline) and AFTER the local
refusal layer (DNS NXDOMAIN / TCP fast refusal) so the ≤2s/target and
≤10s scan-budget acceptance can be compared mechanically.
#>
[CmdletBinding()]
param(
    [string[]]$Targets = @('huggingface.co', 'github.com', 'pypi.org'),
    [int]$TimeoutSeconds = 20,
    [string]$ResultPath = ''
)

$ErrorActionPreference = 'Continue'
$rows = New-Object System.Collections.ArrayList
$sw = [System.Diagnostics.Stopwatch]::StartNew()

function Add-Row([string]$client, [string]$target, [int]$elapsedMs, [string]$status) {
    [void]$rows.Add([ordered]@{
        client = $client
        target = $target
        elapsed_ms = $elapsedMs
        status = $status
    })
}

foreach ($t in $Targets) {
    # 1) Test-NetConnection
    $s = [System.Diagnostics.Stopwatch]::StartNew()
    try {
        $r = Test-NetConnection -ComputerName $t -Port 443 -WarningAction SilentlyContinue
        $ok = [bool]$r.TcpTestSucceeded
    }
    catch {
        $ok = $false
    }
    $s.Stop()
    Add-Row 'Test-NetConnection' $t ([int]$s.ElapsedMilliseconds) $(if ($ok) { 'ok' } else { 'failed' })

    # 2) curl.exe
    $s.Restart()
    curl.exe -m $TimeoutSeconds -sS -o NUL -w '%{http_code}' "https://$t" 2>$null | Out-Null
    $code = $LASTEXITCODE
    $s.Stop()
    Add-Row 'curl' $t ([int]$s.ElapsedMilliseconds) $(if ($code -eq 0) { 'ok' } else { "failed(exit=$code)" })

    # 3) Invoke-WebRequest
    $s.Restart()
    try {
        $resp = Invoke-WebRequest -Uri "https://$t" -TimeoutSec $TimeoutSeconds -UseBasicParsing
        $ok = ($resp.StatusCode -ge 200 -and $resp.StatusCode -lt 400)
    }
    catch {
        $ok = $false
    }
    $s.Stop()
    Add-Row 'Invoke-WebRequest' $t ([int]$s.ElapsedMilliseconds) $(if ($ok) { 'ok' } else { 'failed' })

    # 4) python requests
    $s.Restart()
    $py = @"
import sys, time
try:
    import requests
    r = requests.get('https://$t', timeout=$TimeoutSeconds)
    print('ok', r.status_code)
except Exception as e:
    print('failed', type(e).__name__)
"@
    $pyOut = ($py | python - 2>&1 | Out-String)
    $s.Stop()
    $pyOk = ($pyOut -match '(?m)^ok ')
    Add-Row 'python-requests' $t ([int]$s.ElapsedMilliseconds) $(if ($pyOk) { 'ok' } else { 'failed' })
}

$sw.Stop()
$summary = [ordered]@{
    schema_version = '0.1.0'
    purpose = 'TER T2.2 W-F12 egress baseline / refusal verification'
    targets = $Targets
    client_timeout_seconds = $TimeoutSeconds
    total_wall_ms = [int]$sw.ElapsedMilliseconds
    rows = $rows
}
$json = $summary | ConvertTo-Json -Depth 5
Write-Output $json
if ($ResultPath) {
    [System.IO.File]::WriteAllText($ResultPath, $json, (New-Object System.Text.UTF8Encoding($false)))
}
