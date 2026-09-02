<#
    S4 VM icacls interactive-elevation comparison test (host-side elevated).
    Runs icacls deny + cleanup on C:\WINDOWS via Start-Process -Verb RunAs
    inside the VM (UAC prompt appears on the VM console - user approves).
    ASCII only.
#>
[CmdletBinding()]
param(
    [string]$OutFile = 'D:\CLI\_windows_high_nist\vm-icacls-test-result.txt'
)

$ErrorActionPreference = 'Stop'
$lines = New-Object System.Collections.Generic.List[string]
$runner = 'D:\CLI\scripts\s4_vm_run_elev.ps1'

$body = @'
$innerLines = @(
    'icacls C:\WINDOWS /deny "AgentUser:(OI)(CI)(W,AD,DE,DC)" 2>&1 | Out-File -Encoding utf8 C:\s4\tools\interactive-icacls-test.txt',
    '"DENY_EXIT=$LASTEXITCODE" | Add-Content -Encoding utf8 C:\s4\tools\interactive-icacls-test.txt',
    'icacls C:\WINDOWS /remove:d "AgentUser" 2>&1 | Add-Content -Encoding utf8 C:\s4\tools\interactive-icacls-test.txt',
    '"REMOVE_EXIT=$LASTEXITCODE" | Add-Content -Encoding utf8 C:\s4\tools\interactive-icacls-test.txt'
)
$inner = $innerLines -join [Environment]::NewLine
Set-Content -Encoding utf8 C:\s4\tools\interactive-icacls-test.ps1 -Value $inner
Remove-Item -LiteralPath 'C:\s4\tools\interactive-icacls-test.txt' -Force -ErrorAction SilentlyContinue
$p = Start-Process -FilePath powershell.exe `
    -ArgumentList @('-NoProfile','-ExecutionPolicy','Bypass','-File','C:\s4\tools\interactive-icacls-test.ps1') `
    -Verb RunAs -PassThru
if (-not $p.WaitForExit(180000)) {
    Write-Output 'RUNAS_TIMEOUT'
    Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
} else {
    Write-Output "RUNAS_EXIT=$($p.ExitCode)"
}
if (Test-Path -LiteralPath 'C:\s4\tools\interactive-icacls-test.txt') {
    Get-Content -LiteralPath 'C:\s4\tools\interactive-icacls-test.txt' | ForEach-Object { Write-Output "TEST: $_" }
} else {
    Write-Output 'TEST_FILE_MISSING'
}
'@

$bodyFile = 'D:\CLI\_windows_high_nist\job-icacls-test-body.ps1'
Set-Content -LiteralPath $bodyFile -Value $body -Encoding utf8
try {
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $runner `
        -JobBodyFile $bodyFile -OutFile $OutFile -TimeoutSeconds 300
    if ($LASTEXITCODE -ne 0) { throw "icacls test job failed (exit $LASTEXITCODE)" }
}
catch {
    $lines.Add("ERROR: $($_.Exception.Message)")
    $lines | Set-Content -LiteralPath $OutFile -Encoding utf8
}
