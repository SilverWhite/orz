<#
    S4 VM read latest run observations (host-side elevated). ASCII only.
#>
[CmdletBinding()]
param(
    [string]$OutFile = 'D:\CLI\_windows_high_nist\vm-read-obs-result.txt'
)

$ErrorActionPreference = 'Stop'
$lines = New-Object System.Collections.Generic.List[string]
$runner = 'D:\CLI\scripts\s4_vm_run_elev.ps1'

$body = @'
$files = Get-ChildItem -LiteralPath 'C:\Users\HL\AppData\Local\Temp' -Recurse -Filter 'windows-native-run-observation-*.json' -ErrorAction SilentlyContinue |
    Sort-Object LastWriteTime -Descending | Select-Object -First 4
foreach ($f in $files) {
    Write-Output "===== $($f.FullName) ($($f.LastWriteTime)) ====="
    try {
        $j = Get-Content -LiteralPath $f.FullName -Raw | ConvertFrom-Json
        Write-Output "outcome=$($j.outcome) exit=$($j.process.exit_code)"
        if ($j.checks) {
            $j.checks.PSObject.Properties | ForEach-Object { Write-Output "CHECK $($_.Name)=$($_.Value)" }
        }
        if ($j.diagnostics) {
            Write-Output '--- diagnostics ---'
            $j.diagnostics | ForEach-Object { Write-Output "DIAG: $_" }
        }
    } catch { Write-Output "PARSE_ERR=$($_.Exception.Message)" }
}
'@

$bodyFile = 'D:\CLI\_windows_high_nist\job-read-obs-body.ps1'
Set-Content -LiteralPath $bodyFile -Value $body -Encoding utf8
try {
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $runner `
        -JobBodyFile $bodyFile -OutFile $OutFile -TimeoutSeconds 300
    if ($LASTEXITCODE -ne 0) { throw "read-obs job failed (exit $LASTEXITCODE)" }
}
catch {
    $lines.Add("ERROR: $($_.Exception.Message)")
    $lines | Set-Content -LiteralPath $OutFile -Encoding utf8
}
