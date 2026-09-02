$ErrorActionPreference = 'Continue'
$OutputEncoding = [System.Text.Encoding]::UTF8
try { [Console]::OutputEncoding = [System.Text.Encoding]::UTF8 } catch { }
$env:Path = [Environment]::GetEnvironmentVariable('Path', 'Machine') + ';' + [Environment]::GetEnvironmentVariable('Path', 'User')
$env:PYTHONPATH = 'C:\s4'

$py = 'C:\Program Files\Python312\python.exe'
$cli = 'C:\s4\scripts\run_windows_native_sandbox_command.py'
$provision = 'C:\Program Files\orz\orz-acaf-provision.exe'
$signer = 'C:\Program Files\orz\orz-signer.exe'
$root = 'C:\Users\AgentUser\AppData\Local\orz\acaf'
$manifest = Join-Path $root 'signer-manifest.json'
$ws = 'C:\workspace\acaf-reprovision'

Write-Output "ACAF_TARGET root=$root manifest=$manifest"
Remove-Item -LiteralPath $ws -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Path $ws -Force | Out-Null
Remove-Item -LiteralPath $root -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Path $root -Force | Out-Null
$marker = Join-Path $ws '.assurance-p2-disposable.json'
'{"schema_version":"0.1.0-draft","purpose":"windows-native-sandbox-probe","allow_container_write_probe":true}' |
    Set-Content -LiteralPath $marker -Encoding ascii

$outTxt = Join-Path $ws 'provision.out.txt'
$errTxt = Join-Path $ws 'provision.err.txt'
$cmdPath = Join-Path $ws 'provision.cmd'
$cmdText = '@echo off' + "`r`n" +
    '"' + $provision + '" "' + $root + '" "' + $manifest + '" > "' + $outTxt + '" 2> "' + $errTxt + '"' + "`r`n" +
    'echo PROVISION_EXIT=%ERRORLEVEL%'
Set-Content -LiteralPath $cmdPath -Value $cmdText -Encoding ascii

$argsList = @(
    '--workspace', $ws,
    '--arm', 'non-admin',
    '--timeout', '120',
    '--output', (Join-Path $ws 'obs.json'),
    '--command', 'cmd.exe', '/d', '/c', $cmdPath
)
& $py $cli @argsList 2>&1 | ForEach-Object { Write-Output ('SBX: ' + $_) }
Write-Output ("SBX_EXIT=" + $LASTEXITCODE)
if (Test-Path -LiteralPath $outTxt) {
    Get-Content -LiteralPath $outTxt -Encoding UTF8 -ErrorAction SilentlyContinue | ForEach-Object { Write-Output ("PROVISION_OUT: " + $_) }
}
if (Test-Path -LiteralPath $errTxt) {
    Get-Content -LiteralPath $errTxt -Encoding UTF8 -ErrorAction SilentlyContinue | ForEach-Object { Write-Output ("PROVISION_ERR: " + $_) }
}

$ok = $false
if ((Test-Path -LiteralPath $manifest) -and (Test-Path -LiteralPath (Join-Path $root 'installation-key.dpapi'))) {
    $m = Get-Content -LiteralPath $manifest -Raw -Encoding UTF8 | ConvertFrom-Json
    $signerHash = (Get-FileHash -LiteralPath $signer -Algorithm SHA256).Hash.ToLowerInvariant()
    $match = ([string]$m.binary_sha256).ToLowerInvariant() -eq $signerHash
    Write-Output ("MANIFEST binary_sha256=" + $m.binary_sha256)
    Write-Output ("SIGNER hash=" + $signerHash)
    Write-Output ("MANIFEST_SIGNER_HASH_MATCH=" + $match)
    Get-ChildItem -LiteralPath $root -Force | ForEach-Object {
        Write-Output ("KEYSTORE_ITEM name=" + $_.Name + " size=" + $(if ($_.PSIsContainer) { 0 } else { $_.Length }))
    }
    $ok = $match
}
Write-Output ("ACAF_PROVISION_OK=" + $ok)
if (-not $ok) {
    exit 1
}

[Environment]::SetEnvironmentVariable('ORZ_ACAF_KEYSTORE', $root, 'Machine')
[Environment]::SetEnvironmentVariable('ORZ_ACAF_MANIFEST', $manifest, 'Machine')
[Environment]::SetEnvironmentVariable('ORZ_ACAF_BINARY', $signer, 'Machine')
[Environment]::SetEnvironmentVariable('ORZ_ACAF_FAIL_CLOSED', '1', 'Machine')
Write-Output ("ENV_MACHINE ORZ_ACAF_KEYSTORE=" + [Environment]::GetEnvironmentVariable('ORZ_ACAF_KEYSTORE', 'Machine'))
Write-Output ("ENV_MACHINE ORZ_ACAF_MANIFEST=" + [Environment]::GetEnvironmentVariable('ORZ_ACAF_MANIFEST', 'Machine'))
Write-Output ("ENV_MACHINE ORZ_ACAF_BINARY=" + [Environment]::GetEnvironmentVariable('ORZ_ACAF_BINARY', 'Machine'))
Write-Output ("ENV_MACHINE ORZ_ACAF_FAIL_CLOSED=" + [Environment]::GetEnvironmentVariable('ORZ_ACAF_FAIL_CLOSED', 'Machine'))
Write-Output 'ACAF_REPROVISION_DONE'
exit 0
