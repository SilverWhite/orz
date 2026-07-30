[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$OutputDirectory,

    [Parameter(Mandatory = $true)]
    [ValidateSet('tool_timeout', 'task_cancel', 'parent_exit')]
    [string]$Scenario,

    [Parameter(Mandatory = $true)]
    [string]$ReleaseMetadataPath,

    [string]$BinaryPath,

    [string]$PythonPath,

    [ValidateRange(1000, 30000)]
    [int]$ToolTimeoutMilliseconds = 3000,

    [ValidateRange(20, 180)]
    [int]$TimeoutSeconds = 60,

    [ValidateRange(0, 30)]
    [int]$ExitGraceSeconds = 10
)

$ErrorActionPreference = 'Stop'
if ($env:OS -ne 'Windows_NT') {
    throw 'The Grok Windows child-tree probe is Windows-only.'
}
$identity = [System.Security.Principal.WindowsIdentity]::GetCurrent()
$principal = New-Object System.Security.Principal.WindowsPrincipal($identity)
if (-not $principal.IsInRole([System.Security.Principal.WindowsBuiltInRole]::Administrator)) {
    throw 'Administrator token required: temporary outbound firewall rules are fail-closed.'
}
if (-not $PythonPath) {
    $PythonPath = (Get-Command python -ErrorAction Stop).Source
}
$python = (Resolve-Path -LiteralPath $PythonPath -ErrorAction Stop).Path
$driver = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot 'run_grok_windows_child_tree_probe.py') -ErrorAction Stop).Path
$release = (Resolve-Path -LiteralPath $ReleaseMetadataPath -ErrorAction Stop).Path
$arguments = @(
    $driver,
    '--output-directory', [System.IO.Path]::GetFullPath($OutputDirectory),
    '--scenario', $Scenario,
    '--release-metadata', $release,
    '--python-path', $python,
    '--tool-timeout-ms', [string]$ToolTimeoutMilliseconds,
    '--timeout-seconds', [string]$TimeoutSeconds,
    '--exit-grace-seconds', [string]$ExitGraceSeconds
)
if ($BinaryPath) {
    $arguments += @('--binary', (Resolve-Path -LiteralPath $BinaryPath -ErrorAction Stop).Path)
}
& $python @arguments
exit $LASTEXITCODE
