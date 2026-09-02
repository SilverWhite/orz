$py = 'C:\Program Files\Python312\python.exe'
$mirrors = @(
    'https://mirrors.aliyun.com/pypi/simple/',
    'https://repo.huaweicloud.com/repository/pypi/simple/'
)
$installed = $false
foreach ($m in $mirrors) {
    Write-Output "PIP mirror=$m"
    & $py -m pip install --disable-pip-version-check --index-url $m jsonschema rfc8785 2>&1 | ForEach-Object { Write-Output "PIP: $_" }
    $code = $LASTEXITCODE
    Write-Output "PIP exit=$code"
    if ($code -eq 0) { $installed = $true; break }
}
if (-not $installed) { exit 1 }

& $py -c "import jsonschema, rfc8785; print('DEPS_OK', jsonschema.__version__)" 2>&1 | ForEach-Object { Write-Output "DEPS: $_" }
Set-Location -LiteralPath 'C:\s4'
& $py -c "from assurance.windows_sandbox import WINDOWS_RUN_ARMS, run_windows_native_sandbox; print('IMPORT_OK', WINDOWS_RUN_ARMS)" 2>&1 | ForEach-Object { Write-Output "IMPORT: $_" }
