#requires -Version 5.1
<#
Fixed bridge launcher for the Structured Operation Protocol v0.1.

Usage:
  Get-Content op.json -Raw | powershell -NoProfile -ExecutionPolicy Bypass -File ops_bridge.ps1 -Endpoint wsl:Ubuntu

v0.1 implements wsl endpoints only. The operation JSON is written to a temp
file (UTF-8, no BOM) and handed to the fixed executor inside the target; no
command text crosses the boundary. Endpoint allowlist: ops-bridges.json.
#>

[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$Endpoint
)

$ErrorActionPreference = 'Stop'
$OutputEncoding = New-Object System.Text.UTF8Encoding($false)
try { [Console]::OutputEncoding = New-Object System.Text.UTF8Encoding($false) } catch { }

function ConvertTo-WslPath([string]$WindowsPath) {
    $p = $WindowsPath.Replace('\', '/')
    if ($p -match '^([A-Za-z]):(/.*)?$') {
        return '/mnt/' + $matches[1].ToLower() + $matches[2]
    }
    throw "cannot translate path to WSL: $WindowsPath"
}

$configPath = Join-Path $PSScriptRoot 'ops-bridges.json'
if (-not (Test-Path -LiteralPath $configPath)) { throw "bridges config not found: $configPath" }
$config = Get-Content -LiteralPath $configPath -Raw -Encoding UTF8 | ConvertFrom-Json
$endpointProp = $config.endpoints.PSObject.Properties[$Endpoint]
if (-not $endpointProp) { throw "unknown bridge endpoint: $Endpoint" }
$ep = $endpointProp.Value
if ($ep.type -ne 'wsl') { throw 'ops_bridge.ps1 v0.1 only implements wsl endpoints' }

$json = [Console]::In.ReadToEnd()
if ([string]::IsNullOrWhiteSpace($json)) { throw 'empty input' }
$tmp = Join-Path $env:TEMP ('ops-op-' + [guid]::NewGuid().ToString('N') + '.json')
[System.IO.File]::WriteAllText($tmp, $json, (New-Object System.Text.UTF8Encoding($false)))

try {
    $linuxTmp = ConvertTo-WslPath $tmp
    $envArgs = @('OPS_SOURCE=bridge:' + $Endpoint)
    if ($env:OPS_ALLOW_ROOTS) {
        $linuxRoots = @($env:OPS_ALLOW_ROOTS -split ';' |
            Where-Object { $_ -and $_.Trim() } |
            ForEach-Object { ConvertTo-WslPath $_.Trim() })
        if ($linuxRoots.Count -gt 0) { $envArgs += 'OPS_ALLOW_ROOTS=' + ($linuxRoots -join ':') }
    }
    if ($env:OPS_AUDIT_LOG) {
        $auditFull = [System.IO.Path]::GetFullPath($env:OPS_AUDIT_LOG)
        $envArgs += 'OPS_AUDIT_LOG=' + (ConvertTo-WslPath $auditFull)
    }
    if ($env:OPS_PROCESS_ALLOW) { $envArgs += 'OPS_PROCESS_ALLOW=' + $env:OPS_PROCESS_ALLOW }
    if ($env:OPS_PROTECTED) {
        $linuxProtected = @($env:OPS_PROTECTED -split ';' |
            Where-Object { $_ -and $_.Trim() } |
            ForEach-Object { ConvertTo-WslPath ([System.IO.Path]::GetFullPath($_.Trim())) })
        if ($linuxProtected.Count -gt 0) { $envArgs += 'OPS_PROTECTED=' + ($linuxProtected -join ':') }
    }
    $wslArgs = @('-d', [string]$ep.target, '--', 'env', $envArgs,
                 [string]$ep.interpreter, [string]$ep.executor, '--op-file', $linuxTmp)
    $output = & wsl.exe @wslArgs 2>&1 | Out-String
    $code = $LASTEXITCODE
    $output
    exit $code
} finally {
    if (Test-Path -LiteralPath $tmp) { Remove-Item -LiteralPath $tmp -Force -ErrorAction SilentlyContinue }
}
