# Formatting gate - fails when `cargo fmt --all -- --check` reports drift.
# Run from any directory; resolves the orz repo root from this script's location.
$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
Push-Location $root
try {
    cargo fmt --all -- --check
    if ($LASTEXITCODE -ne 0) {
        Write-Error "rustfmt drift detected. Run 'cargo fmt --all' and commit the result."
        exit $LASTEXITCODE
    }
    Write-Host "rustfmt check passed."
} finally {
    Pop-Location
}
