[CmdletBinding()]
param(
    [string]$BinaryPath,

    [string]$ReleaseMetadataPath
)

$ErrorActionPreference = 'Stop'
$repoRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
$lockPath = if ($ReleaseMetadataPath) {
    (Resolve-Path -LiteralPath $ReleaseMetadataPath -ErrorAction Stop).Path
} else {
    Join-Path $repoRoot 'upstream\grok-build.lock.json'
}
$lock = Get-Content -LiteralPath $lockPath -Raw -Encoding UTF8 | ConvertFrom-Json
$expected = $lock.binary_release
if ($null -eq $expected) {
    throw "Release metadata does not contain binary_release: $lockPath"
}

if (-not $BinaryPath) {
    $relative = $expected.installed_path.Replace('/', '\')
    $BinaryPath = Join-Path $repoRoot $relative
}

$report = [ordered]@{
    schema_version = '0.1.0'
    checked_at = (Get-Date).ToUniversalTime().ToString('o')
    lock_path = $lockPath
    binary_path = $BinaryPath
    installed = $false
    valid = $false
    checks = [ordered]@{}
    observed = [ordered]@{}
    limitations = @(
        'Authenticode validity confirms the local signature chain at check time, not source correspondence.',
        'The released binary build ID is not proven to correspond to the open-source build_repo_commit or SOURCE_REV.',
        'A release receipt alone proves local identity only; project-default selection is determined by the checked-in default lock.',
        'This command performs no login, model request, tool execution, update, or network access.'
    )
}

if (-not (Test-Path -LiteralPath $BinaryPath -PathType Leaf)) {
    $report | ConvertTo-Json -Depth 8
    exit 2
}

$resolved = (Resolve-Path -LiteralPath $BinaryPath).Path
$file = Get-Item -LiteralPath $resolved
$sha256 = (Get-FileHash -LiteralPath $resolved -Algorithm SHA256).Hash.ToLowerInvariant()
$md5 = (Get-FileHash -LiteralPath $resolved -Algorithm MD5).Hash.ToLowerInvariant()
$stream = [System.IO.File]::OpenRead($resolved)
try {
    $first = $stream.ReadByte()
    $second = $stream.ReadByte()
} finally {
    $stream.Dispose()
}
$peHeader = '{0:X2}{1:X2}' -f $first, $second
$signature = Get-AuthenticodeSignature -LiteralPath $resolved
$versionOutput = (& $resolved --version 2>&1 | Out-String).Trim()
$expectedVersion = "grok $($expected.version) ($($expected.build_id))"

$signerCommonName = $null
if ($signature.SignerCertificate) {
    $match = [regex]::Match($signature.SignerCertificate.Subject, '(?:^|,\s*)CN=([^,]+)')
    if ($match.Success) {
        $signerCommonName = $match.Groups[1].Value
    }
}

$report.installed = $true
$report.binary_path = $resolved
$report.observed = [ordered]@{
    bytes = $file.Length
    pe_header = $peHeader
    md5 = $md5
    sha256 = $sha256
    authenticode_status = $signature.Status.ToString()
    signer_common_name = $signerCommonName
    version_output = $versionOutput
}
$report.checks = [ordered]@{
    bytes_match = ($file.Length -eq $expected.bytes)
    pe_header_match = ($peHeader -eq '4D5A')
    md5_match = ($md5 -eq $expected.md5)
    sha256_match = ($sha256 -eq $expected.sha256)
    authenticode_valid = ($signature.Status.ToString() -eq $expected.authenticode_status)
    signer_match = ($signerCommonName -eq $expected.signer_common_name)
    version_match = ($versionOutput -eq $expectedVersion)
}
$report.valid = -not ($report.checks.Values -contains $false)
$report | ConvertTo-Json -Depth 8
if ($report.valid) { exit 0 }
exit 1
