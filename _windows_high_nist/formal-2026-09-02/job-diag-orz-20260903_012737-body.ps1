$ErrorActionPreference = 'Continue'
$OutputEncoding = [System.Text.Encoding]::UTF8
try { [Console]::OutputEncoding = [System.Text.Encoding]::UTF8 } catch { }

function Dump-OrzEvents {
    param([string]$LogName)
    $since = (Get-Date).AddMinutes(-240)
    try {
        $evs = @(Get-WinEvent -FilterHashtable @{ LogName = $LogName; StartTime = $since } -ErrorAction SilentlyContinue |
            Where-Object { $_.Id -in @(1000, 1001, 1002, 1026) -or $_.ProviderName -match 'Application Error|Windows Error Reporting' } |
            Where-Object { $_.Message -match 'orz|0xc0000142|3221225794|dll init|DLL' } |
            Select-Object -First 15)
        foreach ($e in $evs) {
            $msg = ($e.Message -replace '\s+', ' ').Trim()
            if ($msg.Length -gt 700) { $msg = $msg.Substring(0, 700) }
            Write-Output ("EVT {0} log={1} id={2} src={3} msg={4}" -f $e.TimeCreated.ToString('yyyy-MM-dd HH:mm:ss'), $LogName, $e.Id, $e.ProviderName, $msg)
        }
        if ($evs.Count -eq 0) {
            Write-Output ("EVT none log={0}" -f $LogName)
        }
    }
    catch {
        Write-Output ("EVT_ERR log={0} err={1}" -f $LogName, $_.Exception.Message)
    }
}

Dump-OrzEvents -LogName Application
Dump-OrzEvents -LogName System

$probeB64 = 'aW1wb3J0IGN0eXBlcwppbXBvcnQgc3lzCgpvdXRfcGF0aCA9IHN5cy5hcmd2WzFdCndpdGggb3BlbihvdXRfcGF0aCwgInciLCBlbmNvZGluZz0idXRmLTgiKSBhcyBmOgogICAgZi53cml0ZSgiU1RBUlRFRFxuIikKCmRsbHMgPSBbCiAgICAid3MyXzMyLmRsbCIsICJ1c2VyMzIuZGxsIiwgImFkdmFwaTMyLmRsbCIsICJiY3J5cHRwcmltaXRpdmVzLmRsbCIsCiAgICAidXNlcmVudi5kbGwiLCAiYmNyeXB0LmRsbCIsICJjcnlwdDMyLmRsbCIsICJvbGVhdXQzMi5kbGwiLAogICAgInNoZWxsMzIuZGxsIiwgImNvbWJhc2UuZGxsIiwgInNlY3VyMzIuZGxsIiwKXQpvdXQgPSBbXQpmb3IgbmFtZSBpbiBkbGxzOgogICAgdHJ5OgogICAgICAgIGN0eXBlcy5XaW5ETEwobmFtZSkKICAgICAgICBvdXQuYXBwZW5kKG5hbWUgKyAiPU9LIikKICAgIGV4Y2VwdCBPU0Vycm9yIGFzIGV4YzoKICAgICAgICBvdXQuYXBwZW5kKG5hbWUgKyAiPUZBSUwgIiArIHN0cihleGMpKQogICAgZXhjZXB0IEV4Y2VwdGlvbiBhcyBleGM6ICAjIG5vcWE6IEJMRTAwMQogICAgICAgIG91dC5hcHBlbmQobmFtZSArICI9RVhDICIgKyByZXByKGV4YykpCgp3aXRoIG9wZW4ob3V0X3BhdGgsICJhIiwgZW5jb2Rpbmc9InV0Zi04IikgYXMgZjoKICAgIGYud3JpdGUoInwiLmpvaW4ob3V0KSArICJcbiIpCnN5cy5leGl0KDApCg=='
[System.IO.File]::WriteAllBytes(
    'C:\s4\tools\diag_orz_probe.py',
    [Convert]::FromBase64String($probeB64)
)
$probePath = 'C:\s4\tools\diag_orz_probe.py'

$py = 'C:\Program Files\Python312\python.exe'
$cli = 'C:\s4\scripts\run_windows_native_sandbox_command.py'
foreach ($case in @(@{ Arm = 'high-nist'; Tag = 'hn' }, @{ Arm = 'control'; Tag = 'ctl' })) {
    $ws = 'C:\workspace\diag-orz-' + $case.Tag
    Remove-Item -LiteralPath $ws -Recurse -Force -ErrorAction SilentlyContinue
    New-Item -ItemType Directory -Path $ws -Force | Out-Null
    Copy-Item -LiteralPath $probePath -Destination (Join-Path $ws 'probe.py') -Force
    $marker = Join-Path $ws '.assurance-p2-disposable.json'
    if (-not (Test-Path -LiteralPath $marker)) {
        '{"schema_version":"0.1.0-draft","purpose":"windows-native-sandbox-probe","allow_container_write_probe":true}' |
            Set-Content -LiteralPath $marker -Encoding ascii
    }
    $obs = Join-Path $ws 'obs.json'
    $res = Join-Path $ws 'probe-results.txt'
    Write-Output ("=== DLL PROBE arm=" + $case.Arm + " ===")
    & $py $cli --workspace $ws --arm $case.Arm --timeout 180 --output $obs `
        --command $py (Join-Path $ws 'probe.py') $res 2>&1 | ForEach-Object { Write-Output ("SBX: " + $_) }
    Write-Output ("SBX_EXIT=" + $LASTEXITCODE)
    if (Test-Path -LiteralPath $res) {
        Write-Output ("PROBE_RESULTS arm=" + $case.Arm)
        Get-Content -LiteralPath $res -Encoding UTF8
    }
    if (Test-Path -LiteralPath $obs) {
        Get-Content -LiteralPath $obs -Raw -Encoding UTF8
    }
}

Write-Output 'DIAG_ORZ_DONE'
exit 0
