<#
    S4 F1 fix: re-provision ACAF keystore for the wall run-user (AgentUser).

    Root cause (2026-09-03 diag): orz-signer starts but immediately dies on
    DPAPI CryptUnprotectData because K_install is scoped to HL (current-user
    DPAPI), while the wall spawns orz/signer as AgentUser.  The ACAF host
    client discards signer stderr, so this surfaced only as
    "pipe closed (os error 232) / signer_unreachable" on the first action
    ticket.

    Fix steps:
      1. Sync orz.exe / orz-signer.exe / orz-acaf-provision.exe (host
         target\release) into guest C:\Program Files\orz and C:\s4\tools,
         hash-verified, old Program Files copies backed up host-side.
      2. One elevated guest job provisions a NEW keystore+manifest under
         C:\workspace\acaf (the hardening-granted AgentUser-writable root;
         the AgentUser profile itself is deny-frozen by the high-nist wall),
         running
         orz-acaf-provision.exe INSIDE the AgentUser run-user sandbox
         (non-admin arm) so the DPAPI blob binds to AgentUser.
      3. Machine env ORZ_ACAF_KEYSTORE / ORZ_ACAF_MANIFEST are repointed to
         the AgentUser paths (ORZ_ACAF_BINARY / FAIL_CLOSED stay).
    Evidence: <OutFile>.  ASCII only.
#>
[CmdletBinding()]
param(
    [string]$OutFile = 'D:\CLI\_windows_high_nist\vm-acaf-reprovision-result.txt'
)

$ErrorActionPreference = 'Stop'
$lines = New-Object System.Collections.Generic.List[string]
$failed = $false

try {
    $vmName = 'win-s4'
    $sec = ConvertTo-SecureString '123456' -AsPlainText -Force
    $cred = New-Object System.Management.Automation.PSCredential('HL', $sec)
    $sess = $null
    $deadline = (Get-Date).AddSeconds(240)
    while (-not $sess -and (Get-Date) -lt $deadline) {
        try {
            $sess = New-PSSession -VMName $vmName -Credential $cred -ErrorAction Stop
        }
        catch {
            Start-Sleep -Seconds 10
        }
    }
    if (-not $sess) {
        throw 'win-s4 not reachable within 240s'
    }
    $backupDir = 'D:\CLI\_windows_high_nist\backup'
    New-Item -ItemType Directory -Path $backupDir -Force | Out-Null

    # ---- 1) sync the trio ------------------------------------------------
    $localRoot = 'D:\CLI\orz\target\release'
    try {
        foreach ($f in @('orz.exe', 'orz-signer.exe', 'orz-acaf-provision.exe')) {
            $local = Join-Path $localRoot $f
            if (-not (Test-Path -LiteralPath $local)) {
                throw "local binary missing: $local"
            }
            $localHash = (Get-FileHash -LiteralPath $local -Algorithm SHA256).Hash
            $lines.Add("SYNC_SRC $f sha=$localHash size=$((Get-Item -LiteralPath $local).Length)")
            foreach ($dstRoot in @('C:\Program Files\orz', 'C:\s4\tools')) {
                $dst = Join-Path $dstRoot $f
                $dstParent = Split-Path -Parent $dst
                Invoke-Command -Session $sess -ArgumentList $dstParent -ScriptBlock {
                    param($DirPath)
                    New-Item -ItemType Directory -Path $DirPath -Force | Out-Null
                }
                $remoteHash = ''
                $remotePresent = Invoke-Command -Session $sess -ArgumentList $dst -ScriptBlock {
                    param($p) (Test-Path -LiteralPath $p)
                }
                if ($remotePresent) {
                    $remoteHash = Invoke-Command -Session $sess -ArgumentList $dst -ScriptBlock {
                        param($p) (Get-FileHash -LiteralPath $p -Algorithm SHA256).Hash
                    }
                }
                if ($remoteHash -eq $localHash) {
                    $lines.Add("SYNC $dst already-current=1")
                    continue
                }
                if ($dstRoot -eq 'C:\Program Files\orz' -and $remotePresent) {
                    $bak = Join-Path $backupDir ("$f.pre-acaf-" + (Get-Date -Format 'yyyyMMdd_HHmmss') + '.bin')
                    Copy-Item -LiteralPath $dst -Destination $bak -FromSession $sess -Force
                    $bakHash = (Get-FileHash -LiteralPath $bak -Algorithm SHA256).Hash
                    if ($bakHash -ne $remoteHash) {
                        throw "backup hash mismatch for $dst"
                    }
                    $lines.Add("BACKUP $dst -> $bak")
                }
                Copy-Item -LiteralPath $local -Destination $dst -ToSession $sess -Force
                $newHash = Invoke-Command -Session $sess -ArgumentList $dst -ScriptBlock {
                    param($p) (Get-FileHash -LiteralPath $p -Algorithm SHA256).Hash
                }
                $ok = ($newHash -eq $localHash)
                $lines.Add("SYNC $dst ok=$ok sha=$newHash")
                if (-not $ok) {
                    throw "sync hash mismatch: $dst"
                }
            }
        }
    }
    finally {
        Remove-PSSession -Session $sess -ErrorAction SilentlyContinue
    }

    # ---- 2/3) guest elevated job: provision as AgentUser + repoint env ----
    $ts = Get-Date -Format 'yyyyMMdd_HHmmss'
    $bodyFile = 'D:\CLI\_windows_high_nist\job-acaf-reprovision-' + $ts + '-body.ps1'
    $jobOut = 'D:\CLI\_windows_high_nist\job-acaf-reprovision-' + $ts + '.txt'
    $body = @'
$ErrorActionPreference = 'Continue'
$OutputEncoding = [System.Text.Encoding]::UTF8
try { [Console]::OutputEncoding = [System.Text.Encoding]::UTF8 } catch { }
$env:Path = [Environment]::GetEnvironmentVariable('Path', 'Machine') + ';' + [Environment]::GetEnvironmentVariable('Path', 'User')
$env:PYTHONPATH = 'C:\s4'

$py = 'C:\Program Files\Python312\python.exe'
$cli = 'C:\s4\scripts\run_windows_native_sandbox_command.py'
$provision = 'C:\Program Files\orz\orz-acaf-provision.exe'
$signer = 'C:\Program Files\orz\orz-signer.exe'
$root = 'C:\workspace\acaf'
$manifest = Join-Path $root 'signer-manifest.json'
$ws = 'C:\workspace\acaf-reprovision'

Write-Output "ACAF_TARGET root=$root manifest=$manifest"
Remove-Item -LiteralPath $ws -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Path $ws -Force | Out-Null
Remove-Item -LiteralPath $root -Recurse -Force -ErrorAction SilentlyContinue
$marker = Join-Path $ws '.assurance-p2-disposable.json'
'{"schema_version":"0.1.0-draft","purpose":"windows-native-sandbox-probe","allow_container_write_probe":true}' |
    Set-Content -LiteralPath $marker -Encoding ascii

$outTxt = Join-Path $ws 'provision.out.txt'
$errTxt = Join-Path $ws 'provision.err.txt'
$cmdPath = Join-Path $ws 'provision.cmd'
$cmdText = '@echo off' + "`r`n" +
    'mkdir "' + $root + '" 2> nul' + "`r`n" +
    '"' + $provision + '" "' + $root + '" "' + $manifest + '" > "' + $outTxt + '" 2> "' + $errTxt + '"' + "`r`n" +
    'echo PROVISION_EXIT=%ERRORLEVEL%'
Set-Content -LiteralPath $cmdPath -Value $cmdText -Encoding ascii

$argsList = @(
    '--workspace', $ws,
    '--arm', 'non-admin',
    '--timeout', '120',
    '--output', (Join-Path $ws 'obs.json'),
    '--command', 'cmd.exe', '/d', '/c', $cmdPath
)
& $py $cli @argsList 2>&1 | ForEach-Object { Write-Output ('SBX: ' + $_) }
Write-Output ("SBX_EXIT=" + $LASTEXITCODE)
if (Test-Path -LiteralPath $outTxt) {
    Get-Content -LiteralPath $outTxt -Encoding UTF8 -ErrorAction SilentlyContinue | ForEach-Object { Write-Output ("PROVISION_OUT: " + $_) }
}
if (Test-Path -LiteralPath $errTxt) {
    Get-Content -LiteralPath $errTxt -Encoding UTF8 -ErrorAction SilentlyContinue | ForEach-Object { Write-Output ("PROVISION_ERR: " + $_) }
}

$ok = $false
if ((Test-Path -LiteralPath $manifest) -and (Test-Path -LiteralPath (Join-Path $root 'installation-key.dpapi'))) {
    $m = Get-Content -LiteralPath $manifest -Raw -Encoding UTF8 | ConvertFrom-Json
    $signerHash = (Get-FileHash -LiteralPath $signer -Algorithm SHA256).Hash.ToLowerInvariant()
    $match = ([string]$m.binary_sha256).ToLowerInvariant() -eq $signerHash
    Write-Output ("MANIFEST binary_sha256=" + $m.binary_sha256)
    Write-Output ("SIGNER hash=" + $signerHash)
    Write-Output ("MANIFEST_SIGNER_HASH_MATCH=" + $match)
    Get-ChildItem -LiteralPath $root -Force | ForEach-Object {
        Write-Output ("KEYSTORE_ITEM name=" + $_.Name + " size=" + $(if ($_.PSIsContainer) { 0 } else { $_.Length }))
    }
    $ok = $match
}
Write-Output ("ACAF_PROVISION_OK=" + $ok)
if (-not $ok) {
    exit 1
}

[Environment]::SetEnvironmentVariable('ORZ_ACAF_KEYSTORE', $root, 'Machine')
[Environment]::SetEnvironmentVariable('ORZ_ACAF_MANIFEST', $manifest, 'Machine')
[Environment]::SetEnvironmentVariable('ORZ_ACAF_BINARY', $signer, 'Machine')
[Environment]::SetEnvironmentVariable('ORZ_ACAF_FAIL_CLOSED', '1', 'Machine')
Write-Output ("ENV_MACHINE ORZ_ACAF_KEYSTORE=" + [Environment]::GetEnvironmentVariable('ORZ_ACAF_KEYSTORE', 'Machine'))
Write-Output ("ENV_MACHINE ORZ_ACAF_MANIFEST=" + [Environment]::GetEnvironmentVariable('ORZ_ACAF_MANIFEST', 'Machine'))
Write-Output ("ENV_MACHINE ORZ_ACAF_BINARY=" + [Environment]::GetEnvironmentVariable('ORZ_ACAF_BINARY', 'Machine'))
Write-Output ("ENV_MACHINE ORZ_ACAF_FAIL_CLOSED=" + [Environment]::GetEnvironmentVariable('ORZ_ACAF_FAIL_CLOSED', 'Machine'))
Write-Output 'ACAF_REPROVISION_DONE'
exit 0
'@

    Set-Content -LiteralPath $bodyFile -Value $body -Encoding utf8
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File 'D:\CLI\scripts\s4_vm_run_elev.ps1' `
        -JobBodyFile $bodyFile -OutFile $jobOut -TimeoutSeconds 600
    $jobExit = $LASTEXITCODE
    $lines.Add("JOB_EXIT=$jobExit")
    if (Test-Path -LiteralPath $jobOut) {
        foreach ($l in (Get-Content -LiteralPath $jobOut -Encoding UTF8)) {
            $lines.Add($l)
        }
    }
    if ($jobExit -ne 0) {
        $failed = $true
    }
}
catch {
    $lines.Add("ERROR: $($_.Exception.Message)")
    $failed = $true
}

$lines.Add("ACAF_REPROVISION_HOST_OK=$(-not $failed)")
$lines | Set-Content -LiteralPath $OutFile -Encoding utf8
Write-Output "resultFile=$OutFile"
if ($failed) {
    exit 1
}
exit 0
