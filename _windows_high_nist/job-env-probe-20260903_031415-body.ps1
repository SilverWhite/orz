$ErrorActionPreference = 'Continue'
$OutputEncoding = [System.Text.Encoding]::UTF8
try { [Console]::OutputEncoding = [System.Text.Encoding]::UTF8 } catch { }

function Get-ToolVersion([string]$Name, [string[]]$ArgList) {
    $found = $false
    foreach ($cand in @($Name, ($Name + '.exe'))) {
        $cmd = Get-Command $cand -ErrorAction SilentlyContinue
        if ($cmd) {
            $found = $true
            try {
                $out = & $cmd.Source @ArgList 2>&1 | Select-Object -First 1
                Write-Output ("TOOL name=" + $Name + " present=1 path=" + $cmd.Source + " version=" + $out)
            }
            catch {
                Write-Output ("TOOL name=" + $Name + " present=1 path=" + $cmd.Source + " version=<run-error>")
            }
            break
        }
    }
    if (-not $found) {
        Write-Output ("TOOL name=" + $Name + " present=0")
    }
}

Write-Output "=== ENV INVENTORY ==="
Write-Output ("WHOAMI=" + (whoami))
Write-Output ("COMPUTER=" + $env:COMPUTERNAME)
$drive = Get-PSDrive -Name C -ErrorAction SilentlyContinue
if ($drive) {
    Write-Output ("DISK_FREE_GB=" + [math]::Round($drive.Free / 1GB, 2))
    Write-Output ("DISK_USED_GB=" + [math]::Round($drive.Used / 1GB, 2))
}

$py = 'C:\Program Files\Python312\python.exe'
Write-Output ("PYTHON_EXE present=" + (Test-Path -LiteralPath $py))
if (Test-Path -LiteralPath $py) {
    & $py --version 2>&1 | ForEach-Object { Write-Output ("PYTHON_VERSION: " + $_) }
    $pkgs = & $py -m pip list --format=freeze 2>&1
    Write-Output "PIP_LIST_FILTERED:"
    $pkgs | Where-Object { $_ -match '^(numpy|pandas|pyarrow|fasttext|scipy|scikit|Pillow|PIL|selenium|beautifulsoup4|bs4|requests|jsonschema|rfc8785|torch|transformers|mteb|primer3|biopython)' } |
        ForEach-Object { Write-Output ("  " + $_) }
    Write-Output ("PIP_OK=" + ($LASTEXITCODE -eq 0))
}

Get-ToolVersion -Name 'node' -ArgList @('--version')
Get-ToolVersion -Name 'npm' -ArgList @('--version')
Get-ToolVersion -Name 'gcc' -ArgList @('--version')
Get-ToolVersion -Name 'g++' -ArgList @('--version')
Get-ToolVersion -Name 'clang' -ArgList @('--version')
Get-ToolVersion -Name 'llc' -ArgList @('--version')
Get-ToolVersion -Name 'make' -ArgList @('--version')
Get-ToolVersion -Name 'git' -ArgList @('--version')
Get-ToolVersion -Name 'R' -ArgList @('--version')
Get-ToolVersion -Name 'Rscript' -ArgList @('--version')
Get-ToolVersion -Name 'ffmpeg' -ArgList @('-version')
Get-ToolVersion -Name '7z' -ArgList @('')
Get-ToolVersion -Name 'curl' -ArgList @('--version')
Get-ToolVersion -Name 'wget' -ArgList @('--version')

Write-Output "MACHINE_PATH:"
[Environment]::GetEnvironmentVariable('Path', 'Machine') -split ';' |
    Where-Object { $_ } | ForEach-Object { Write-Output ("  " + $_) }

foreach ($dir in @('C:\Program Files\orz', 'C:\s4\tools', 'C:\s4\scripts',
        'C:\s4\_windows_high_nist\run', 'C:\s4\_windows_high_nist\agent-tasks-tb2.1',
        'C:\workspace', 'C:\s4\data', 'C:\s4\wheelhouse')) {
    if (Test-Path -LiteralPath $dir) {
        Write-Output ("DIR present=1 path=" + $dir)
        Get-ChildItem -LiteralPath $dir -Force -ErrorAction SilentlyContinue |
            Select-Object -First 12 |
            ForEach-Object {
                if ($_.PSIsContainer) {
                    Write-Output ("  SUBDIR name=" + $_.Name)
                }
                else {
                    Write-Output ("  FILE name=" + $_.Name + " size=" + $_.Length)
                }
            }
    }
    else {
        Write-Output ("DIR present=0 path=" + $dir)
    }
}

Write-Output "ENV_PROBE_DONE"
exit 0
