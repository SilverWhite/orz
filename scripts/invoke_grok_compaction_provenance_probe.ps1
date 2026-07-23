[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$OutputDirectory,

    [string]$ReleaseMetadataPath,

    [string]$BinaryPath,

    [string]$PythonPath,

    [ValidateRange(20, 300)]
    [int]$TimeoutSeconds = 90
)

$ErrorActionPreference = 'Stop'
if ($env:OS -ne 'Windows_NT') {
    throw 'The Grok compaction provenance probe is Windows-only.'
}
$identity = [System.Security.Principal.WindowsIdentity]::GetCurrent()
$principal = New-Object System.Security.Principal.WindowsPrincipal($identity)
if (-not $principal.IsInRole([System.Security.Principal.WindowsBuiltInRole]::Administrator)) {
    throw 'Administrator token required: temporary outbound firewall rules are fail-closed.'
}
if (-not $PythonPath) {
    $PythonPath = (Get-Command python -ErrorAction Stop).Source
}
if (-not $ReleaseMetadataPath) {
    $ReleaseMetadataPath = Join-Path (Split-Path $PSScriptRoot -Parent) 'upstream\grok-build.lock.json'
}
$python = (Resolve-Path -LiteralPath $PythonPath -ErrorAction Stop).Path
$driver = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot 'run_grok_compaction_provenance_probe.py') -ErrorAction Stop).Path
$release = (Resolve-Path -LiteralPath $ReleaseMetadataPath -ErrorAction Stop).Path
$arguments = @(
    $driver,
    '--output-directory', [System.IO.Path]::GetFullPath($OutputDirectory),
    '--release-metadata', $release,
    '--python-path', $python,
    '--timeout-seconds', [string]$TimeoutSeconds
)
if ($BinaryPath) {
    $arguments += @('--binary', (Resolve-Path -LiteralPath $BinaryPath -ErrorAction Stop).Path)
}
& $python @arguments
exit $LASTEXITCODE
