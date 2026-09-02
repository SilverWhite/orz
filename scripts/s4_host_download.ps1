<#
    S4 host-side downloader: fetches the official Python 3.12 amd64
    installer (python.org, then huaweicloud mirror) into
    D:\CLI\_windows_high_nist\tools and writes a result file.
    ASCII only.
#>
[CmdletBinding()]
param(
    [string]$OutFile = 'D:\CLI\_windows_high_nist\host-download-result.txt'
)

$ErrorActionPreference = 'Continue'
$lines = New-Object System.Collections.Generic.List[string]
$dir = 'D:\CLI\_windows_high_nist\tools'
$target = Join-Path $dir 'python-3.12.10-amd64.exe'
New-Item -ItemType Directory -Path $dir -Force | Out-Null
Remove-Item -LiteralPath $target -Force -ErrorAction SilentlyContinue

$urls = @(
    'https://www.python.org/ftp/python/3.12.10/python-3.12.10-amd64.exe',
    'https://mirrors.huaweicloud.com/python/3.12.10/python-3.12.10-amd64.exe'
)

$ok = $false
foreach ($u in $urls) {
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    & curl.exe -L --max-time 300 --connect-timeout 20 -sS -o $target $u
    $code = $LASTEXITCODE
    $sw.Stop()
    $size = 0
    if (Test-Path -LiteralPath $target) { $size = (Get-Item -LiteralPath $target).Length }
    $lines.Add("URL=$u curl_exit=$code seconds=$([math]::Round($sw.Elapsed.TotalSeconds,1)) size=$size")
    if ($code -eq 0 -and $size -gt 20000000) {
        $ok = $true
        break
    }
    Remove-Item -LiteralPath $target -Force -ErrorAction SilentlyContinue
}
$lines.Add("DOWNLOAD_OK=$ok")
$lines | Set-Content -LiteralPath $OutFile -Encoding utf8
