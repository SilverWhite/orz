<#
    S4 VM pip deps + minimal assurance stub (host-side elevated).
      A) replace C:\s4\assurance\__init__.py with a minimal stub
      B) pip install jsonschema rfc8785 via CN mirror
      C) verify imports for the sandbox run path
    ASCII only.
#>
[CmdletBinding()]
param(
    [string]$OutFile = 'D:\CLI\_windows_high_nist\vm-pip-result.txt'
)

$ErrorActionPreference = 'Stop'
$lines = New-Object System.Collections.Generic.List[string]
$runner = 'D:\CLI\scripts\s4_vm_run_elev.ps1'

$body = @'
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
'@

$bodyFile = 'D:\CLI\_windows_high_nist\job-pip-body.ps1'
Set-Content -LiteralPath $bodyFile -Value $body -Encoding utf8
try {
    $sec = ConvertTo-SecureString '123456' -AsPlainText -Force
    $cred = New-Object System.Management.Automation.PSCredential('HL', $sec)
    $sess = New-PSSession -VMName 'win-s4' -Credential $cred
    Copy-Item -LiteralPath 'D:\CLI\scripts\s4_assurance_stub_init.py' `
        -Destination 'C:\s4\assurance\__init__.py' -ToSession $sess -Force
    Remove-PSSession -Session $sess
    $lines.Add('STUB_WRITTEN=yes')
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $runner `
        -JobBodyFile $bodyFile -OutFile $OutFile -TimeoutSeconds 1200
    if ($LASTEXITCODE -ne 0) { throw "pip job failed (exit $LASTEXITCODE)" }
}
catch {
    $lines.Add("ERROR: $($_.Exception.Message)")
    $lines | Set-Content -LiteralPath $OutFile -Encoding utf8
}
