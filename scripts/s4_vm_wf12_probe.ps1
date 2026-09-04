<#
    TER T2.2 (W-F12) guest enforcement-probe runner (host-side elevated).

    Runs run_enforcement_probe.ps1 (high-nist wall child) inside win-s4 via
    the fixed guest-job runner, in two wall forms with the same egress
    allowlist, and echoes both wall-child result JSONs (wf12_timings_ms):

      Run A (ac-enabled): AppContainer empty-capability compartment baseline.
          Evidence that per-IP allow rules are swallowed by the AC network
          compartment, so allowlist_reachable FAILs by design whenever an
          egress allowlist is set (AC mode is not a valid allowlist wall).
      Run B (noappcontainer): production agent wiring (2026-09-03 ruling):
          non-admin + LOW IL + Job + TEMP redirect + host blockoutbound +
          per-IP allow rules.  This is the TER T2.2 acceptance run: external
          targets fail fast (<=2s) and the allowlist stays reachable.

    Job exits 0 only when Run B spawns compliant and its probe passes.
    ASCII only inside the guest body.
#>
[CmdletBinding()]
param(
    [string]$OutFile = 'D:\CLI\_windows_high_nist\vm-wf12-probe-result.txt'
)

$ErrorActionPreference = 'Stop'
$lines = New-Object System.Collections.Generic.List[string]
$failed = $false

try {
    $ts = Get-Date -Format 'yyyyMMdd_HHmmss'
    $bodyFile = 'D:\CLI\_windows_high_nist\job-wf12-probe-' + $ts + '-body.ps1'
    $jobOut = 'D:\CLI\_windows_high_nist\job-wf12-probe-' + $ts + '.txt'
    $body = @'
$ErrorActionPreference = 'Continue'
$OutputEncoding = [System.Text.Encoding]::UTF8
try { [Console]::OutputEncoding = [System.Text.Encoding]::UTF8 } catch { }
$env:PYTHONIOENCODING = 'utf-8'
$env:Path = [Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [Environment]::GetEnvironmentVariable('Path','User')
$allowIp = '221.204.163.76'
$wsRoot = 'C:\workspace\wf12-ep'
New-Item -ItemType Directory -Path $wsRoot -Force | Out-Null
$tn = Test-NetConnection -ComputerName $allowIp -Port 443 -WarningAction SilentlyContinue
Write-Output ("DIRECT_ALLOWLIST_REACHABLE=" + $tn.TcpTestSucceeded)
$ep = 'C:\s4\_windows_high_nist\run\run_enforcement_probe.ps1'
$script:acExit = 1
$script:noacExit = 1
$script:acSpawnOk = 'False'
$script:noacSpawnOk = 'False'

function Invoke-ProbeArm {
    param(
        [string]$Label,
        [string]$Sub,
        [switch]$NoAc
    )
    $ws = Join-Path $wsRoot $Sub
    New-Item -ItemType Directory -Path $ws -Force | Out-Null
    $res = Join-Path $ws 'enforcement.json'
    if (Test-Path -LiteralPath $res) { Remove-Item -LiteralPath $res -Force }
    $argsList = @('-Arm', 'high-nist', '-Workspace', $ws, '-AllowlistIp', $allowIp, '-ResultPath', $res, '-TimeoutSeconds', 240)
    if ($NoAc) {
        $argsList += '-NoAppcontainer'
    }
    $code = 1
    try {
        & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $ep @argsList 2>&1 |
            ForEach-Object { Write-Output ("ENF-${Label}: " + $_) }
        $code = $LASTEXITCODE
    }
    catch {
        Write-Output ("ERROR-$Label=" + $_.Exception.Message)
        $code = 1
    }
    if ($null -eq $code) { $code = 1 }
    Write-Output ("EXIT_$Label=$code")
    if ($NoAc) {
        $script:noacExit = [int]$code
    }
    else {
        $script:acExit = [int]$code
    }
    $obs = Join-Path $ws 'windows-native-run-observation-high-nist.json'
    $spawnOk = 'False'
    if (Test-Path -LiteralPath $obs) {
        try {
            $o = Get-Content -LiteralPath $obs -Raw -Encoding UTF8 | ConvertFrom-Json
            $spawnOk = [string]$o.outcome -eq 'compliant'
            Write-Output ("SPAWN_${Label}_OUTCOME=" + $o.outcome)
            Write-Output ("SPAWN_${Label}_APPCONTAINER_MODE=" + $o.appcontainer_mode)
        }
        catch { }
    }
    Write-Output ("SPAWN_${Label}_OK=" + $spawnOk)
    if ($NoAc) {
        $script:noacSpawnOk = [string]$spawnOk
    }
    else {
        $script:acSpawnOk = [string]$spawnOk
    }
    $dbg = Join-Path $ws '.enforcement-probe-high-nist.debug.log'
    if (Test-Path -LiteralPath $dbg) {
        Write-Output ("DEBUG_${Label}_BEGIN")
        Get-Content -LiteralPath $dbg -Raw -Encoding UTF8
        Write-Output ("DEBUG_${Label}_END")
    }
    $childErr = Join-Path $ws '.sandbox-child-stderr.log'
    if (Test-Path -LiteralPath $childErr) {
        Write-Output ("CHILD_STDERR_${Label}_BEGIN")
        Get-Content -LiteralPath $childErr -Raw -Encoding UTF8
        Write-Output ("CHILD_STDERR_${Label}_END")
    }
    $childOut = Join-Path $ws '.sandbox-child-stdout.log'
    if (Test-Path -LiteralPath $childOut) {
        Write-Output ("CHILD_STDOUT_${Label}_BEGIN")
        Get-Content -LiteralPath $childOut -Raw -Encoding UTF8
        Write-Output ("CHILD_STDOUT_${Label}_END")
    }
    if (Test-Path -LiteralPath $obs) {
        Write-Output ("OBS_${Label}_BEGIN")
        Get-Content -LiteralPath $obs -Raw -Encoding UTF8
        Write-Output ("OBS_${Label}_END")
    }
    if (Test-Path -LiteralPath $res) {
        Write-Output ("RESULT_${Label}_BEGIN")
        Get-Content -LiteralPath $res -Raw -Encoding UTF8
        Write-Output ("RESULT_${Label}_END")
    }
    else {
        Write-Output ("RESULT_${Label}_MISSING")
    }
}

Write-Output 'RUN_A_AC_ENABLED_BEGIN'
Invoke-ProbeArm -Label 'AC' -Sub 'ac-enabled'
Write-Output 'RUN_A_AC_ENABLED_END'
Write-Output 'RUN_B_NOAC_BEGIN'
Invoke-ProbeArm -Label 'NOAC' -Sub 'noac' -NoAc
Write-Output 'RUN_B_NOAC_END'
Write-Output 'RUN_F_EGRESS_BEGIN'
$wsG = Join-Path $wsRoot 'egress'
New-Item -ItemType Directory -Path $wsG -Force | Out-Null
$markerG = Join-Path $wsG '.assurance-p2-disposable.json'
@{ schema_version = '0.1.0-draft'; purpose = 'windows-native-sandbox-probe'; allow_container_write_probe = $true } |
    ConvertTo-Json | Set-Content -LiteralPath $markerG -Encoding ascii
$egressArgs = @(
    'C:\s4\scripts\run_windows_native_sandbox_command.py',
    '--workspace', $wsG,
    '--arm', 'high-nist',
    '--no-appcontainer',
    '--allowlist-ip', $allowIp,
    '--timeout', 180,
    '--output', (Join-Path $wsG 'obs.json'),
    '--command',
    'powershell.exe', '-NoProfile', '-ExecutionPolicy', 'Bypass', '-File',
    'C:\s4\_windows_high_nist\run\run_wf12_egress_probe.ps1',
    '-TimeoutSeconds', '20'
)
try {
    python.exe @egressArgs 2>&1 | ForEach-Object { Write-Output ("EGRESS: " + $_) }
    Write-Output ("EGRESS_EXIT=" + $LASTEXITCODE)
}
catch {
    Write-Output ("EGRESS_ERROR=" + $_.Exception.Message)
    Write-Output 'EGRESS_EXIT=1'
}
$egOut = Join-Path $wsG '.sandbox-child-stdout.log'
if (Test-Path -LiteralPath $egOut) {
    Write-Output 'EGRESS_CHILD_OUT_BEGIN'
    Get-Content -LiteralPath $egOut -Raw -Encoding UTF8
    Write-Output 'EGRESS_CHILD_OUT_END'
}
$egErr = Join-Path $wsG '.sandbox-child-stderr.log'
if (Test-Path -LiteralPath $egErr) {
    Write-Output 'EGRESS_CHILD_ERR_BEGIN'
    Get-Content -LiteralPath $egErr -Raw -Encoding UTF8
    Write-Output 'EGRESS_CHILD_ERR_END'
}
Write-Output 'RUN_F_EGRESS_END'
Write-Output 'RUN_G_DNS_REFUSAL_BEGIN'
$dnsIdx = 0
$savedDns = @()
$daemon = $null
try {
    $net = Get-NetIPConfiguration -ErrorAction SilentlyContinue |
        Where-Object { $_.IPv4DefaultGateway -and $_.NetAdapter.Status -eq 'Up' } |
        Select-Object -First 1
    if ($net) {
        $dnsIdx = [int]$net.InterfaceIndex
        $savedDns = @(
            Get-DnsClientServerAddress -InterfaceIndex $dnsIdx -AddressFamily IPv4 -ErrorAction SilentlyContinue |
                Where-Object { $_.ServerAddresses } |
                Select-Object -ExpandProperty ServerAddresses
        )
        Write-Output ("DNS_IDX=" + $dnsIdx)
        Write-Output ("DNS_SAVED=" + ($savedDns -join ','))
    }
    $daemon = Start-Process -FilePath 'python.exe' `
        -ArgumentList @('C:\s4\_windows_high_nist\wf12\dns_refusal.py', '--port', '53') `
        -WindowStyle Hidden -PassThru
    Start-Sleep -Seconds 3
    if ($dnsIdx -gt 0) {
        Set-DnsClientServerAddress -InterfaceIndex $dnsIdx -ServerAddresses '127.0.0.1' -ErrorAction Stop
    }
    & ipconfig.exe /flushdns 2>&1 | Out-Null
    $swD = [System.Diagnostics.Stopwatch]::StartNew()
    $nx = $false
    try {
        $rd = Resolve-DnsName -Name 'huggingface.co' -Type A -DnsOnly -ErrorAction SilentlyContinue
        $nx = ($null -eq $rd -or @($rd).Count -eq 0)
    }
    catch {
        Write-Output ("RESOLVE_ERR=" + $_.Exception.Message)
    }
    $swD.Stop()
    Write-Output ("RESOLVE_NXDOMAIN=" + $nx)
    Write-Output ("RESOLVE_ELAPSED_MS=" + $swD.ElapsedMilliseconds)
    $wsH = Join-Path $wsRoot 'egress-post'
    New-Item -ItemType Directory -Path $wsH -Force | Out-Null
    $markerH = Join-Path $wsH '.assurance-p2-disposable.json'
    @{ schema_version = '0.1.0-draft'; purpose = 'windows-native-sandbox-probe'; allow_container_write_probe = $true } |
        ConvertTo-Json | Set-Content -LiteralPath $markerH -Encoding ascii
    $egressPostArgs = @(
        'C:\s4\scripts\run_windows_native_sandbox_command.py',
        '--workspace', $wsH,
        '--arm', 'high-nist',
        '--no-appcontainer',
        '--allowlist-ip', $allowIp,
        '--timeout', 180,
        '--output', (Join-Path $wsH 'obs.json'),
        '--command',
        'powershell.exe', '-NoProfile', '-ExecutionPolicy', 'Bypass', '-File',
        'C:\s4\_windows_high_nist\run\run_wf12_egress_probe.ps1',
        '-TimeoutSeconds', '20'
    )
    try {
        python.exe @egressPostArgs 2>&1 | ForEach-Object { Write-Output ("EGRESS_POST: " + $_) }
        Write-Output ("EGRESS_POST_EXIT=" + $LASTEXITCODE)
    }
    catch {
        Write-Output ("EGRESS_POST_ERROR=" + $_.Exception.Message)
        Write-Output 'EGRESS_POST_EXIT=1'
    }
    $egOutH = Join-Path $wsH '.sandbox-child-stdout.log'
    if (Test-Path -LiteralPath $egOutH) {
        Write-Output 'EGRESS_POST_CHILD_OUT_BEGIN'
        Get-Content -LiteralPath $egOutH -Raw -Encoding UTF8
        Write-Output 'EGRESS_POST_CHILD_OUT_END'
    }
}
finally {
    if ($daemon -and -not $daemon.HasExited) {
        Stop-Process -Id $daemon.Id -Force -ErrorAction SilentlyContinue
    }
    if ($dnsIdx -gt 0) {
        try {
            if ($savedDns.Count -gt 0) {
                Set-DnsClientServerAddress -InterfaceIndex $dnsIdx -ServerAddresses $savedDns -ErrorAction Stop
                Write-Output 'DNS_RESTORED=True'
            }
            else {
                Set-DnsClientServerAddress -InterfaceIndex $dnsIdx -ResetServerAddresses -ErrorAction SilentlyContinue
                Write-Output 'DNS_RESTORED=True'
            }
        }
        catch {
            Write-Output ("DNS_RESTORE_ERR=" + $_.Exception.Message)
        }
    }
    & ipconfig.exe /flushdns 2>&1 | Out-Null
}
Write-Output 'RUN_G_DNS_REFUSAL_END'
Write-Output 'WF12_FWRULES_BEGIN'
netsh advfirewall firewall show rule name=all 2>&1 |
    Select-String -Pattern 'GSA-P2-Native-Sandbox' -Context 0,3 | ForEach-Object { $_.ToString() }
Write-Output 'WF12_FWRULES_END'
Write-Output 'WF12_PROBE_DONE'
$noacLog = Join-Path (Join-Path $wsRoot 'noac') '.sandbox-child-stdout.log'
$noacNetOk = $false
if (Test-Path -LiteralPath $noacLog) {
    $noacLogText = Get-Content -LiteralPath $noacLog -Raw -Encoding UTF8
    $noacNetOk = ($noacLogText -match 'PASS  network_blocked') -and
        ($noacLogText -match 'PASS  allowlist_reachable') -and
        ($noacLogText -match 'PASS  metadata_blocked') -and
        ($noacLogText -match 'WF12 allowlist_reachable_ms=')
}
Write-Output ("NOAC_NETWORK_OK=" + $noacNetOk)
if (-not $noacNetOk -or $script:noacSpawnOk -ne 'True') {
    Write-Output 'WF12_NOAC_JOB_FAIL=1'
    exit 1
}
Write-Output 'WF12_NOAC_JOB_FAIL=0'
exit 0
'@

    Set-Content -LiteralPath $bodyFile -Value $body -Encoding utf8
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File 'D:\CLI\scripts\s4_vm_run_elev.ps1' `
        -JobBodyFile $bodyFile -OutFile $jobOut -TimeoutSeconds 900
    $jobExit = $LASTEXITCODE
    $lines.Add("JOB_EXIT=$jobExit")
    if (Test-Path -LiteralPath $jobOut) {
        foreach ($l in (Get-Content -LiteralPath $jobOut -Encoding UTF8)) {
            $lines.Add($l)
        }
    }
    if ($jobExit -ne 0 -or ($lines | Where-Object { $_ -match '^EXIT=[1-9][0-9]*$' })) {
        $failed = $true
    }
}
catch {
    $lines.Add("ERROR: $($_.Exception.Message)")
    $failed = $true
}

$lines.Add("WF12_PROBE_HOST_OK=$(-not $failed)")
$lines | Set-Content -LiteralPath $OutFile -Encoding utf8
Write-Output "resultFile=$OutFile"
if ($failed) {
    exit 1
}
exit 0
