$ErrorActionPreference = 'Continue'
$OutputEncoding = [System.Text.Encoding]::UTF8
try { [Console]::OutputEncoding = [System.Text.Encoding]::UTF8 } catch { }

$py = 'C:\Program Files\Python312\python.exe'
$orz = 'C:\Program Files\orz\orz.exe'
$cli = 'C:\s4\scripts\run_windows_native_sandbox_command.py'
$bogusKey = 'sk-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA'

$cases = @(
    @{ Arm = 'control';   Label = 'ctl-python' },
    @{ Arm = 'control';   Label = 'ctl-orz' },
    @{ Arm = 'non-admin'; Label = 'nonadmin-orz' },
    @{ Arm = 'high-nist'; Label = 'hn-python' },
    @{ Arm = 'high-nist'; Label = 'hn-orz' }
)

foreach ($case in $cases) {
    $ws = 'C:\workspace\diag-sweep-' + $case.Label
    Remove-Item -LiteralPath $ws -Recurse -Force -ErrorAction SilentlyContinue
    New-Item -ItemType Directory -Path $ws -Force | Out-Null
    $marker = Join-Path $ws '.assurance-p2-disposable.json'
    '{"schema_version":"0.1.0-draft","purpose":"windows-native-sandbox-probe","allow_container_write_probe":true}' |
        Set-Content -LiteralPath $marker -Encoding ascii
    $obs = Join-Path $ws 'obs.json'
    $argsList = @(
        '--workspace', $ws,
        '--arm', $case.Arm,
        '--timeout', '60',
        '--output', $obs,
        '--env', ('ORZ_DEEPSEEK_API_KEY=' + $bogusKey),
        '--env', 'ORZ_MAIN_AGENT_MODEL=deepseek-v4-flash'
    )
    if ($case.Label -match 'python$') {
        $argsList += @('--command', $py, '-c', 'import sys; sys.exit(0)')
    }
    else {
        $argsList += @(
            '--command', $orz,
            '-p', 'Reply with exactly: ok',
            '--real',
            '--allow-write',
            '--allow-shell',
            '--allow-network',
            '--max-wallclock', '30',
            '--run-root', $ws
        )
    }
    Write-Output ('=== CASE ' + $case.Label + ' arm=' + $case.Arm + ' ===')
    & $py $cli @argsList 2>&1 | ForEach-Object { Write-Output ('SBX: ' + $_) }
    Write-Output ('SBX_EXIT=' + $LASTEXITCODE)
}

Write-Output 'DIAG_ORZ_SWEEP_DONE'
exit 0
