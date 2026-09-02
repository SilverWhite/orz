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
