<#
    S4 diagnostic: F1 signer deployment + in-wall spawn verification.

    One SYSTEM guest job runs:
      (0) inventory: Program Files\orz + C:\s4\tools exe hashes, ORZ_ACAF env
          (machine/user/process), manifest + keystore listing.
      (1) direct elevated signer spawn probes (immediate + idle-then-request)
          capturing signer stderr/exit (the ACAF host client discards signer
          stderr, which is why F1 only surfaced as "pipe closed, os error
          232").
      (2) sandbox signer spawn probes through the native sandbox CLI:
          ctl / non-admin / high-nist-noac, each doing a stdio round-trip
          (initialize_session) with signer manifest+keystore env.
    Evidence: <OutDir>\diag-signer-f1-<ts>.txt.  ASCII only.
#>
[CmdletBinding()]
param(
    [string]$OutDir = 'D:\CLI\_windows_high_nist\formal-2026-09-02'
)

$ErrorActionPreference = 'Stop'
$ts = Get-Date -Format 'yyyyMMdd_HHmmss'
$bodyFile = Join-Path $OutDir "job-diag-signer-f1-$ts-body.ps1"
$outFile = Join-Path $OutDir "diag-signer-f1-$ts.txt"

$body = @'
$ErrorActionPreference = 'Continue'
$OutputEncoding = [System.Text.Encoding]::UTF8
try { [Console]::OutputEncoding = [System.Text.Encoding]::UTF8 } catch { }

$py = 'C:\Program Files\Python312\python.exe'
$cli = 'C:\s4\scripts\run_windows_native_sandbox_command.py'

function Get-FirstEnv([string]$Name) {
    $v = [Environment]::GetEnvironmentVariable($Name, 'Machine')
    if ([string]::IsNullOrWhiteSpace($v)) {
        $v = [Environment]::GetEnvironmentVariable($Name, 'User')
    }
    if ([string]::IsNullOrWhiteSpace($v)) {
        $v = [Environment]::GetEnvironmentVariable($Name, 'Process')
    }
    return $v
}

# ---- (0) inventory ---------------------------------------------------------
Write-Output '=== SIGNER INVENTORY ==='
Write-Output ("WHOAMI=" + (whoami))
$pf = 'C:\Program Files\orz'
if (Test-Path -LiteralPath $pf) {
    Get-ChildItem -LiteralPath $pf -File -Filter '*.exe' | Sort-Object Name | ForEach-Object {
        $h = (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash
        Write-Output ("PF_FILE name=" + $_.Name + " size=" + $_.Length + " sha=" + $h)
    }
}
else {
    Write-Output 'PF_ORZ_MISSING'
}
if (Test-Path -LiteralPath 'C:\s4\tools') {
    Get-ChildItem -LiteralPath 'C:\s4\tools' -File -Filter '*.exe' | Sort-Object Name | ForEach-Object {
        $h = (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash
        Write-Output ("TOOLS_FILE name=" + $_.Name + " size=" + $_.Length + " sha=" + $h)
    }
}
foreach ($name in @('ORZ_ACAF_MANIFEST', 'ORZ_ACAF_KEYSTORE', 'ORZ_ACAF_BINARY', 'ORZ_ACAF_FAIL_CLOSED', 'ORZ_SIGNER_MANIFEST', 'ORZ_SIGNER_KEYSTORE_ROOT')) {
    $mv = [Environment]::GetEnvironmentVariable($name, 'Machine')
    $uv = [Environment]::GetEnvironmentVariable($name, 'User')
    $pv = [Environment]::GetEnvironmentVariable($name, 'Process')
    Write-Output ("ENV_MACHINE " + $name + "=" + $mv)
    Write-Output ("ENV_USER " + $name + "=" + $uv)
    Write-Output ("ENV_PROC " + $name + "=" + $pv)
}

$mref = Get-FirstEnv 'ORZ_ACAF_MANIFEST'
$kref = Get-FirstEnv 'ORZ_ACAF_KEYSTORE'
$bref = Get-FirstEnv 'ORZ_ACAF_BINARY'
Write-Output ("ACAF_RESOLVED manifest=" + $mref)
Write-Output ("ACAF_RESOLVED keystore=" + $kref)
Write-Output ("ACAF_RESOLVED binary=" + $bref)

$manifestCandidates = @()
if ($mref) { $manifestCandidates += $mref }
$manifestCandidates += 'C:\Program Files\orz\signer-manifest.json'
$manifestCandidates += 'C:\Users\HL\AppData\Local\orz\acaf\signer-manifest.json'
$manifestCandidates += 'C:\Users\AgentUser\AppData\Local\orz\acaf\signer-manifest.json'
$seen = @{}
foreach ($mc in ($manifestCandidates | Select-Object -Unique)) {
    if ($mc -and (Test-Path -LiteralPath $mc) -and -not $seen.ContainsKey($mc)) {
        $seen[$mc] = $true
        Write-Output ("MANIFEST path=" + $mc)
        Get-Content -LiteralPath $mc -Encoding UTF8 | ForEach-Object { Write-Output ("MANIFEST_LINE " + $_) }
    }
}

$keystoreCandidates = @()
if ($kref) { $keystoreCandidates += $kref }
$keystoreCandidates += 'C:\Users\HL\AppData\Local\orz\acaf'
$keystoreCandidates += 'C:\Users\AgentUser\AppData\Local\orz\acaf'
foreach ($kc in ($keystoreCandidates | Select-Object -Unique)) {
    if ($kc -and (Test-Path -LiteralPath $kc)) {
        Write-Output ("KEYSTORE path=" + $kc)
        Get-ChildItem -LiteralPath $kc -Force -ErrorAction SilentlyContinue | ForEach-Object {
            Write-Output ("KEYSTORE_ITEM name=" + $_.Name + " isdir=" + $_.PSIsContainer + " size=" + $(if ($_.PSIsContainer) { 0 } else { $_.Length }))
        }
    }
}

$signerExe = 'C:\Program Files\orz\orz-signer.exe'
if (-not (Test-Path -LiteralPath $signerExe)) { $signerExe = 'C:\s4\tools\orz-signer.exe' }
Write-Output ("SIGNER_EXE=" + $signerExe + " exists=" + (Test-Path -LiteralPath $signerExe))

$reqText = '{"id":1,"method":"initialize_session","params":{"session_id":"DIAG-S1","agent_id":"diag","goal_version":0,"goal_digest":"0000000000000000000000000000000000000000000000000000000000000000","policy_revision":0}}' + "`r`n" + '{"id":2,"method":"sign_command_exec_v1","params":{"session_id":"DIAG-S1","canonical_arguments_sha256":"1111111111111111111111111111111111111111111111111111111111111111","resolved_target_sha256":"2222222222222222222222222222222222222222222222222222222222222222"}}'
$probeRoot = 'C:\workspace\diag-signer-probe'
Remove-Item -LiteralPath $probeRoot -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Path $probeRoot -Force | Out-Null

# F2: C:\app is a per-case junction to the sandbox workspace (same wiring
# the agent runner performs per task).  A real C:\app directory was found
# integrity-denied for the high-nist LOW-IL child; the workspace subdir is
# writable, so /app must resolve into the workspace.
$appRoot = 'C:\app'
if (Test-Path -LiteralPath $appRoot) {
    Get-ChildItem -LiteralPath $appRoot -Force -ErrorAction SilentlyContinue |
        Remove-Item -Recurse -Force -ErrorAction SilentlyContinue
    & cmd.exe /d /c ('rmdir "' + $appRoot + '"') 2>&1 | Out-Null
}
Write-Output ("APP_ROOT_CLEARED=1")
$labelProbeDir = 'C:\workspace\diag-label-probe'
New-Item -ItemType Directory -Path $labelProbeDir -Force | Out-Null
Write-Output '=== LABEL INSPECT ==='
foreach ($p in @($labelProbeDir)) {
    Write-Output ("ICACLS " + $p)
    & icacls.exe $p | ForEach-Object { Write-Output ("  " + $_) }
}
Write-Output 'ICACLS C:\workspace'
& icacls.exe 'C:\workspace' | ForEach-Object { Write-Output ("  " + $_) }

function New-SignerCmdFile {
    param(
        [string]$Path,
        [string]$Label,
        [string]$ManifestPath,
        [string]$KeystoreRoot,
        [switch]$IdleFirst,
        [string]$ExtraDir
    )
    $dir = if ($ExtraDir) { $ExtraDir } else { $probeRoot }
    $reqP = Join-Path $dir ($Label + '.req.txt')
    Set-Content -LiteralPath $reqP -Value $reqText -Encoding ascii
    $outP = Join-Path $dir ($Label + '.out.txt')
    $errP = Join-Path $dir ($Label + '.err.txt')
    $exeLine = '"' + $signerExe + '"'
    $idle = ''
    if ($IdleFirst) {
        $idle = "ping -n 16 127.0.0.1 > nul`r`n"
    }
    $cmd = '@echo off' + "`r`n" +
        'set ORZ_SIGNER_MANIFEST=' + $ManifestPath + "`r`n" +
        'set ORZ_SIGNER_KEYSTORE_ROOT=' + $KeystoreRoot + "`r`n" +
        $idle +
        'type "' + $reqP + '" | ' + $exeLine + ' > "' + $outP + '" 2> "' + $errP + '"' + "`r`n" +
        'echo SIGNER_PROBE_EXIT=%ERRORLEVEL%' + "`r`n"
    Set-Content -LiteralPath $Path -Value $cmd -Encoding ascii
}

function Invoke-DirectProbe {
    param(
        [string]$Label,
        [string]$CmdFile,
        [int]$TimeoutSec = 25
    )
    Write-Output ("=== SIGNER DIRECT " + $Label + " ===")
    $hostOut = Join-Path $probeRoot ($Label + '-host.out')
    $hostErr = Join-Path $probeRoot ($Label + '-host.err')
    $p = Start-Process -FilePath 'cmd.exe' -ArgumentList @('/d', '/c', ('"' + $CmdFile + '"')) -PassThru -WindowStyle Hidden -RedirectStandardOutput $hostOut -RedirectStandardError $hostErr
    if (-not $p.WaitForExit($TimeoutSec * 1000)) {
        try { & taskkill.exe /PID $p.Id /T /F 2>&1 | Out-Null } catch { }
        Write-Output ("PROBE_TIMEOUT " + $Label)
    }
    Write-Output ("PROBE_DIRECT_EXIT " + $Label + " code=" + $p.ExitCode)
    $outP = Join-Path $probeRoot ($Label + '.out.txt')
    $errP = Join-Path $probeRoot ($Label + '.err.txt')
    if (Test-Path -LiteralPath $outP) {
        Get-Content -LiteralPath $outP -Encoding UTF8 -ErrorAction SilentlyContinue | ForEach-Object { Write-Output ("SIGNER_OUT " + $_) }
    }
    if (Test-Path -LiteralPath $errP) {
        Get-Content -LiteralPath $errP -Encoding UTF8 -ErrorAction SilentlyContinue | ForEach-Object { Write-Output ("SIGNER_ERR " + $_) }
    }
    if (Test-Path -LiteralPath $hostErr) {
        Get-Content -LiteralPath $hostErr -Encoding UTF8 -ErrorAction SilentlyContinue | ForEach-Object { Write-Output ("HOST_ERR " + $_) }
    }
}

# ---- (1) direct elevated probes -------------------------------------------
if ($mref -and $kref -and (Test-Path -LiteralPath $signerExe)) {
    $c1 = Join-Path $probeRoot 'direct-immediate.cmd'
    New-SignerCmdFile -Path $c1 -Label 'direct-immediate' -ManifestPath $mref -KeystoreRoot $kref
    Invoke-DirectProbe -Label 'direct-immediate' -CmdFile $c1
    $c2 = Join-Path $probeRoot 'direct-idle.cmd'
    New-SignerCmdFile -Path $c2 -Label 'direct-idle' -ManifestPath $mref -KeystoreRoot $kref -IdleFirst
    Invoke-DirectProbe -Label 'direct-idle' -CmdFile $c2
}
else {
    Write-Output 'DIRECT_SIGNER_PROBES_SKIPPED (acaf env or binary missing)'
}

# ---- (2) sandbox wall probes ----------------------------------------------
$cases = @(
    @{ Arm = 'control';   Label = 'ctl-signer'; Signer = $true },
    @{ Arm = 'non-admin'; Label = 'nonadmin-signer'; Signer = $true },
    @{ Arm = 'non-admin'; Label = 'nonadmin-app'; App = $true },
    @{ Arm = 'high-nist'; Label = 'hn-noac-signer'; Signer = $true },
    @{ Arm = 'high-nist'; Label = 'hn-noac-signer-idle'; Signer = $true; IdleFirst = $true },
    @{ Arm = 'high-nist'; Label = 'hn-noac-app'; App = $true }
)

foreach ($case in $cases) {
    $ws = 'C:\workspace\diag-signer-' + $case.Label
    Remove-Item -LiteralPath $ws -Recurse -Force -ErrorAction SilentlyContinue
    New-Item -ItemType Directory -Path $ws -Force | Out-Null
    $marker = Join-Path $ws '.assurance-p2-disposable.json'
    '{"schema_version":"0.1.0-draft","purpose":"windows-native-sandbox-probe","allow_container_write_probe":true}' |
        Set-Content -LiteralPath $marker -Encoding ascii
    $sw = $null
    $sMan = ''
    $sKey = ''
    $cmdPath = ''
    if ($case.Signer) {
        $sMan = Get-FirstEnv 'ORZ_ACAF_MANIFEST'
        $sKey = Get-FirstEnv 'ORZ_ACAF_KEYSTORE'
        if (-not $sMan -or -not $sKey) {
            Write-Output ('=== CASE ' + $case.Label + ' skipped (no acaf env) ===')
            continue
        }
        $cmdPath = Join-Path $ws ('signer-' + $case.Label + '.cmd')
        if ($case.IdleFirst) { $sw = @{ IdleFirst = $true } }
        New-SignerCmdFile -Path $cmdPath -Label ('wall-' + $case.Label) -ManifestPath $sMan -KeystoreRoot $sKey -ExtraDir $ws @sw
    }
    elseif ($case.App) {
        if (Test-Path -LiteralPath $appRoot) {
            & cmd.exe /d /c ('rmdir "' + $appRoot + '"') 2>&1 | Out-Null
        }
        & cmd.exe /d /c ('mklink /J "' + $appRoot + '" "' + $ws + '"') 2>&1 | ForEach-Object { Write-Output ('MKLINK ' + $_) }
        $probeFile = Join-Path $appRoot ('diag-probe-' + $case.Label + '.txt')
        $ownFile = Join-Path $ws ('own-' + $case.Label + '.txt')
        $wsRootFile = 'C:\workspace\diag-wsroot-' + $case.Label + '.txt'
        $cmdPath = Join-Path $ws ('app-' + $case.Label + '.cmd')
        $cmdText = '@echo off' + "`r`n" +
            'echo own-ok ' + $case.Label + ' > "' + $ownFile + '" 2> nul' + "`r`n" +
            'if exist "' + $ownFile + '" (echo OWN_WRITE=OK) else (echo OWN_WRITE=FAIL)' + "`r`n" +
            'echo app-ok ' + $case.Label + ' > "' + $probeFile + '" 2> "' + (Join-Path $ws ('apperr-' + $case.Label + '.txt')) + '"' + "`r`n" +
            'if exist "' + $probeFile + '" (echo APP_WRITE=OK) else (echo APP_WRITE=FAIL)' + "`r`n" +
            'echo wsroot-ok ' + $case.Label + ' > "' + $wsRootFile + '" 2> "' + (Join-Path $ws ('wsroterr-' + $case.Label + '.txt')) + '"' + "`r`n" +
            'if exist "' + $wsRootFile + '" (echo WSROOT_WRITE=OK) else (echo WSROOT_WRITE=FAIL)' + "`r`n"
        Set-Content -LiteralPath $cmdPath -Value $cmdText -Encoding ascii
    }
    $argsList = @(
        '--workspace', $ws,
        '--arm', $case.Arm,
        '--timeout', '60',
        '--output', (Join-Path $ws 'obs.json')
    )
    if ($case.Signer) {
        $argsList += @('--env', ('ORZ_SIGNER_MANIFEST=' + $sMan))
        $argsList += @('--env', ('ORZ_SIGNER_KEYSTORE_ROOT=' + $sKey))
    }
    if ($case.Arm -eq 'high-nist') {
        $argsList += '--no-appcontainer'
        $argsList += @('--allowlist-ip', '221.204.163.76')
    }
    $argsList += @('--command', 'cmd.exe', '/d', '/c', $cmdPath)
    Write-Output ('=== CASE ' + $case.Label + ' arm=' + $case.Arm + ' ===')
    if ($case.App) {
        Write-Output ('ICACLS ' + $ws)
        & icacls.exe $ws | ForEach-Object { Write-Output ('  ' + $_) }
    }
    & $py $cli @argsList 2>&1 | ForEach-Object { Write-Output ('SBX: ' + $_) }
    Write-Output ('SBX_EXIT=' + $LASTEXITCODE)
    foreach ($suffix in @('out.txt', 'err.txt')) {
        $p = Join-Path $ws ('wall-' + $case.Label + '.' + $suffix)
        if (Test-Path -LiteralPath $p) {
            Write-Output ('WALL_' + $suffix.ToUpper() + ' ' + $case.Label)
            Get-Content -LiteralPath $p -Encoding UTF8 -ErrorAction SilentlyContinue
        }
    }
    if ($case.App) {
        $probeFile = Join-Path $appRoot ('diag-probe-' + $case.Label + '.txt')
        $ownFile = Join-Path $ws ('own-' + $case.Label + '.txt')
        $wsRootFile = 'C:\workspace\diag-wsroot-' + $case.Label + '.txt'
        Write-Output ('APP_PROBE ' + $case.Label + ' own=' + (Test-Path -LiteralPath $ownFile) +
            ' app=' + (Test-Path -LiteralPath $probeFile) +
            ' wsroot=' + (Test-Path -LiteralPath $wsRootFile))
        foreach ($errName in @(('apperr-' + $case.Label + '.txt'), ('wsroterr-' + $case.Label + '.txt'))) {
            $ep = Join-Path $ws $errName
            if (Test-Path -LiteralPath $ep) {
                Write-Output ('APP_ERRFILE ' + $errName)
                Get-Content -LiteralPath $ep -Encoding Default -ErrorAction SilentlyContinue |
                    ForEach-Object { Write-Output ('APP_ERR ' + $_) }
            }
        }
    }
}

Write-Output 'DIAG_SIGNER_F1_DONE'
exit 0
'@

Set-Content -LiteralPath $bodyFile -Value $body -Encoding utf8
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File 'D:\CLI\scripts\s4_vm_run_elev.ps1' `
    -JobBodyFile $bodyFile -OutFile $outFile -TimeoutSeconds 900
$exit = $LASTEXITCODE
Write-Output "DIAG_EXIT=$exit"
Write-Output "DIAG_OUT=$outFile"
if (Test-Path -LiteralPath $outFile) {
    Get-Content -LiteralPath $outFile -Encoding UTF8
}
exit $exit
