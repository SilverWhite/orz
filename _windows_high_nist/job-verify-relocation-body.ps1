[Environment]::SetEnvironmentVariable('ORZ_ACAF_BINARY', 'C:\Program Files\orz\orz-signer.exe', 'Machine')
Write-Output "BINARY=$([Environment]::GetEnvironmentVariable('ORZ_ACAF_BINARY','Machine'))"
$manifestPath = 'C:\Users\HL\AppData\Local\orz\acaf\signer-manifest.json'
$manifest = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
$hash = (Get-FileHash -LiteralPath 'C:\Program Files\orz\orz-signer.exe' -Algorithm SHA256).Hash
Write-Output "SIGNER_HASH=$hash"
Write-Output "MANIFEST_HASH=$($manifest.binary_sha256)"
if ($hash -ne $manifest.binary_sha256) { Write-Output 'SIGNER_HASH_MISMATCH'; exit 1 }
Write-Output 'SIGNER_HASH_MATCH=yes'
$env:Path = [Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [Environment]::GetEnvironmentVariable('Path','User')
$py = & python -c "import sys; print(sys.executable); print(sys.version)" 2>&1
Write-Output "PY=$($py -join ' ')"
foreach ($f in @('orz.exe','orz-signer.exe','orz-acaf-provision.exe')) {
    Write-Output "PF_$f=$(Test-Path -LiteralPath (Join-Path 'C:\Program Files\orz' $f))"
}
