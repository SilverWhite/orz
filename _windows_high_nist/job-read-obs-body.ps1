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
