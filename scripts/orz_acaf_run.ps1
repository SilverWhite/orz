param(
    # Directory containing orz.exe / orz-signer.exe / orz-acaf-provision.exe
    # (defaults to the workspace debug build).
    [string]$BinDir = (Join-Path $PSScriptRoot '..\orz\target\debug'),
    # Keystore + manifest root (defaults to the per-user local app data).
    [string]$Root = (Join-Path $env:LOCALAPPDATA 'orz\acaf'),
    # Explicit shadow mode (ORZ_ACAF_FAIL_CLOSED=0) instead of the
    # production default (fail-closed enforced).
    [switch]$Shadow,
    # Remaining arguments are forwarded to orz unchanged.
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$OrzArgs
)

$ErrorActionPreference = 'Stop'

$provision = Join-Path $BinDir 'orz-acaf-provision.exe'
$orz = Join-Path $BinDir 'orz.exe'
$signer = Join-Path $BinDir 'orz-signer.exe'

if (-not (Test-Path -LiteralPath $provision)) {
    throw "provision tool not found: $provision (build first: cargo build -p orz-bin)"
}
if (-not (Test-Path -LiteralPath $orz)) {
    throw "orz not found: $orz"
}
if (-not (Test-Path -LiteralPath $signer)) {
    throw "signer not found: $signer"
}

New-Item -ItemType Directory -Path $Root -Force | Out-Null
$manifest = Join-Path $Root 'signer-manifest.json'

Write-Host "ACAF provisioning (keystore=$Root manifest=$manifest signer=$signer)"
& $provision $Root $manifest
if ($LASTEXITCODE -ne 0) {
    throw "ACAF provisioning failed (exit $LASTEXITCODE)"
}

$env:ORZ_ACAF_KEYSTORE = $Root
$env:ORZ_ACAF_MANIFEST = $manifest
$env:ORZ_ACAF_BINARY = $signer
$env:ORZ_ACAF_FAIL_CLOSED = if ($Shadow) { '0' } else { '1' }

Write-Host "launching orz (fail-closed=$(-not $Shadow)): $orz $($OrzArgs -join ' ')"
& $orz @OrzArgs
exit $LASTEXITCODE
