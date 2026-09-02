<#
    S4 AppContainer credential injection end-to-end probe (host-side elevated).

    Runs one SYSTEM job inside win-s4 that:
      1. reads the real DeepSeek credential as AgentUser (non-admin sandbox)
         and dumps the raw blob to a disposable workspace file,
      2. canonicalizes the key into a BOM-less env JSON (never on argv),
      3. launches a high-nist AppContainer sandbox child with
         ORZ_DEEPSEEK_API_KEY injected via --env-file,
      4. the child copies the env value to a UTF-8 file (no secret printed),
      5. SYSTEM compares the two files (hash + bytes),
      6. deletes every file that ever held the key.

    Proves the per-run injection channel the agent round will use.  ASCII only.
#>
[CmdletBinding()]
param(
    [string]$OutFile = 'D:\CLI\_windows_high_nist\vm-cred-inject-result.txt'
)

$ErrorActionPreference = 'Stop'
$vmName = 'win-s4'
$runner = 'D:\CLI\scripts\s4_vm_run_elev.ps1'
$exe = 'D:\CLI\_windows_high_nist\tools\cred_probe.exe'
$sandboxCliSrc = 'D:\CLI\scripts\run_windows_native_sandbox_command.py'
$lines = New-Object System.Collections.Generic.List[string]

try {
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
    try {
        Invoke-Command -Session $sess -ScriptBlock {
            New-Item -ItemType Directory -Path 'C:\s4\tools' -Force | Out-Null
            New-Item -ItemType Directory -Path 'C:\s4\scripts' -Force | Out-Null
        }
        foreach ($t in @(
            @{ Src = $exe; Dst = 'C:\s4\tools\cred_probe.exe' },
            @{ Src = $sandboxCliSrc; Dst = 'C:\s4\scripts\run_windows_native_sandbox_command.py' }
        )) {
            Copy-Item -LiteralPath $t.Src -Destination $t.Dst -ToSession $sess -Force
            $srcHash = (Get-FileHash -LiteralPath $t.Src -Algorithm SHA256).Hash
            $dstHash = Invoke-Command -Session $sess -ArgumentList $t.Dst -ScriptBlock {
                param($p) (Get-FileHash -LiteralPath $p -Algorithm SHA256).Hash
            }
            if ($srcHash -ne $dstHash) {
                throw "sync hash mismatch: $($t.Dst)"
            }
        }
    }
    finally {
        Remove-PSSession -Session $sess -ErrorAction SilentlyContinue
    }

    $body = @'
$env:Path = [Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [Environment]::GetEnvironmentVariable('Path','User')
$env:PYTHONPATH = 'C:\s4'
$ErrorActionPreference = 'Continue'
$root = 'C:\workspace\cred-inject'
New-Item -ItemType Directory -Path $root -Force | Out-Null
$py = 'C:\Program Files\Python312\python.exe'
$sandboxCli = 'C:\s4\scripts\run_windows_native_sandbox_command.py'
$credExe = 'C:\s4\tools\cred_probe.exe'
$errors = New-Object System.Collections.ArrayList
$utf8NoBom = New-Object System.Text.UTF8Encoding($false)
$summary = [ordered]@{
    ran_at = (Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ')
    bootstrap_ok = $false
    ac_env_copy_ok = $false
    match = $false
    bytes_match = $false
    errors = New-Object System.Collections.ArrayList
}

function New-ProbeWorkspace {
    param([string]$Name)
    $wd = Join-Path $root $Name
    New-Item -ItemType Directory -Path $wd -Force | Out-Null
    $wdExe = Join-Path $wd 'cred_probe.exe'
    Copy-Item -LiteralPath $credExe -Destination $wdExe -Force
    if (-not (Test-Path -LiteralPath $wdExe)) {
        throw "cred_probe.exe missing in $wd"
    }
    $marker = Join-Path $wd '.assurance-p2-disposable.json'
    @{
        schema_version = '0.1.0-draft'
        purpose = 'windows-native-sandbox-probe'
        allow_container_write_probe = $true
    } | ConvertTo-Json | Set-Content -LiteralPath $marker -Encoding ascii
    return $wd
}

# ---- Step 1: AgentUser (non-admin) reads the real credential + raw dump ----
Write-Output '===== STEP 1: AgentUser non-admin read + dump ====='
$bootstrapDir = New-ProbeWorkspace 'bootstrap'
$dumpPath = Join-Path $bootstrapDir 'key.blob'
$readRes = Join-Path $bootstrapDir 'cred-read.json'
$bootstrapObs = Join-Path $root 'obs-bootstrap.json'
$bootstrapArgs = @(
    '--workspace', $bootstrapDir,
    '--arm', 'non-admin',
    '--timeout', 240,
    '--output', $bootstrapObs,
    '--env', "CRED_PROBE_DUMP=$dumpPath",
    '--command',
    (Join-Path $bootstrapDir 'cred_probe.exe'), $readRes
)
& $py $sandboxCli @bootstrapArgs 2>&1 | ForEach-Object { Write-Output "BOOT: $_" }
$bootExit = $LASTEXITCODE
Write-Output "BOOT_EXIT=$bootExit"
if ((Test-Path -LiteralPath $readRes) -and (Test-Path -LiteralPath $dumpPath)) {
    $r = Get-Content -LiteralPath $readRes -Raw -Encoding UTF8 | ConvertFrom-Json
    Write-Output "BOOT_READ ok=$($r.ok) winerror=$($r.winerror) persist=$($r.persist) blob_bytes=$($r.blob_bytes) dump_ok=$($r.dump_ok)"
    if ($r.ok -and $r.dump_ok -and [int]$r.blob_bytes -gt 0) {
        $summary.bootstrap_ok = $true
        Write-Output 'CRED_BOOTSTRAP_OK=True'
    }
    else {
        [void]$errors.Add('bootstrap read/dump incomplete')
        [void]$summary.errors.Add('bootstrap read/dump incomplete')
    }
}
else {
    [void]$errors.Add("bootstrap result missing (bootExit=$bootExit)")
    [void]$summary.errors.Add("bootstrap result missing (bootExit=$bootExit)")
}

# ---- Step 2: canonical key text + env JSON (never on argv) ----
Write-Output '===== STEP 2: canonicalize key + env JSON ====='
$canonPath = Join-Path $root 'key-canon.txt'
$envFile = Join-Path $root 'inject-env.json'
if ($summary.bootstrap_ok) {
    $raw = [System.IO.File]::ReadAllBytes($dumpPath)
    $keyText = [System.Text.Encoding]::Unicode.GetString($raw)
    $keyText = $keyText.TrimEnd([char]0)
    [System.IO.File]::WriteAllText($canonPath, $keyText, $utf8NoBom)
    $envObj = [ordered]@{
        ORZ_DEEPSEEK_API_KEY = $keyText
    }
    [System.IO.File]::WriteAllText($envFile, ($envObj | ConvertTo-Json -Compress), $utf8NoBom)
    Write-Output "KEY_CHARS=$($keyText.Length)"
}

# ---- Step 3: high-nist AppContainer child receives env + copies it out ----
Write-Output '===== STEP 3: high-nist AC env injection + env copy ====='
$acDir = New-ProbeWorkspace 'ac'
$envCopyPath = Join-Path $acDir 'env.txt'
$envRes = Join-Path $acDir 'env-result.json'
$acObs = Join-Path $root 'obs-ac.json'
if ($summary.bootstrap_ok) {
    $acEnvFile = Join-Path $root 'ac-env.json'
    $acEnvObj = [ordered]@{
        ORZ_DEEPSEEK_API_KEY = $keyText
        CRED_PROBE_ENV_COPY = "ORZ_DEEPSEEK_API_KEY;$envCopyPath"
    }
    [System.IO.File]::WriteAllText($acEnvFile, ($acEnvObj | ConvertTo-Json -Compress), $utf8NoBom)
    $acArgs = @(
        '--workspace', $acDir,
        '--arm', 'high-nist',
        '--timeout', 240,
        '--output', $acObs,
        '--env-file', $acEnvFile,
        '--command',
        (Join-Path $acDir 'cred_probe.exe'), $envRes
    )
    & $py $sandboxCli @acArgs 2>&1 | ForEach-Object { Write-Output "AC: $_" }
    $acExit = $LASTEXITCODE
    Write-Output "AC_EXIT=$acExit"
    if ((Test-Path -LiteralPath $envRes) -and (Test-Path -LiteralPath $envCopyPath)) {
        $er = Get-Content -LiteralPath $envRes -Raw -Encoding UTF8 | ConvertFrom-Json
        Write-Output "AC_ENV_COPY ok=$($er.ok) winerror=$($er.winerror) bytes=$($er.blob_bytes) mode=$($er.mode)"
        if ($er.ok -and [int]$er.blob_bytes -gt 0) {
            $summary.ac_env_copy_ok = $true
            Write-Output 'AC_ENV_COPY_OK=True'
        }
        else {
            [void]$errors.Add('AC env copy failed')
            [void]$summary.errors.Add('AC env copy failed')
        }
    }
    else {
        [void]$errors.Add("AC env copy result missing (acExit=$acExit)")
        [void]$summary.errors.Add("AC env copy result missing (acExit=$acExit)")
    }
}

# ---- Step 4: compare canonical key vs AC-side env copy ----
Write-Output '===== STEP 4: compare ====='
if ($summary.bootstrap_ok -and $summary.ac_env_copy_ok) {
    $h1 = (Get-FileHash -LiteralPath $canonPath -Algorithm SHA256).Hash
    $h2 = (Get-FileHash -LiteralPath $envCopyPath -Algorithm SHA256).Hash
    $b1 = [System.IO.File]::ReadAllBytes($canonPath)
    $b2 = [System.IO.File]::ReadAllBytes($envCopyPath)
    $summary.match = ($h1 -eq $h2)
    $summary.bytes_match = ([System.Convert]::ToBase64String($b1) -eq [System.Convert]::ToBase64String($b2))
    Write-Output "INJECT_HASH_MATCH=$($summary.match)"
    Write-Output "INJECT_BYTES_MATCH=$($summary.bytes_match)"
    Write-Output "INJECT_CANON_BYTES=$($b1.Length) INJECT_AC_BYTES=$($b2.Length)"
}

# ---- Step 5: secret cleanup (blobs/env files never leave the VM) ----
Write-Output '===== STEP 5: secret cleanup ====='
foreach ($f in @($dumpPath, $canonPath, $envFile, (Join-Path $root 'ac-env.json'), $envCopyPath)) {
    if (Test-Path -LiteralPath $f) {
        Remove-Item -LiteralPath $f -Force -ErrorAction SilentlyContinue
    }
}
$secretLeft = @()
foreach ($f in @($dumpPath, $canonPath, $envFile, (Join-Path $root 'ac-env.json'), $envCopyPath)) {
    if (Test-Path -LiteralPath $f) { $secretLeft += $f }
}
Write-Output "SECRET_FILES_LEFT=$($secretLeft.Count)"
if ($secretLeft.Count -gt 0) {
    foreach ($f in $secretLeft) { Write-Output "SECRET_LEFT: $f" }
    [void]$errors.Add('secret cleanup incomplete')
}

# ---- Evidence: copy secret-free JSONs to root ----
$bootstrapResOut = Join-Path $root 'bootstrap-read.json'
$envResOut = Join-Path $root 'ac-env-result.json'
if (Test-Path -LiteralPath $readRes) {
    Copy-Item -LiteralPath $readRes -Destination $bootstrapResOut -Force
}
if (Test-Path -LiteralPath $envRes) {
    Copy-Item -LiteralPath $envRes -Destination $envResOut -Force
}
$summary.errors = $errors
$summary | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath (Join-Path $root 'inject-summary.json') -Encoding utf8

if ($errors.Count -gt 0) {
    Write-Output "CRED_INJECT_ERRORS=$($errors.Count)"
    foreach ($e in $errors) {
        Write-Output "CRED_INJECT_ERROR: $e"
    }
    Write-Output 'CRED_INJECT_DONE'
    exit 1
}
if (-not ($summary.match -and $summary.bytes_match)) {
    Write-Output 'CRED_INJECT_ERRORS=1'
    Write-Output 'CRED_INJECT_ERROR: hash or bytes mismatch'
    Write-Output 'CRED_INJECT_DONE'
    exit 1
}
Write-Output 'CRED_INJECT_ERRORS=0'
Write-Output 'CRED_INJECT_DONE'
exit 0
'@

    $bodyFile = 'D:\CLI\_windows_high_nist\formal-2026-09-02\job-cred-inject-body.ps1'
    $resFile = 'D:\CLI\_windows_high_nist\formal-2026-09-02\job-cred-inject-result.txt'
    Set-Content -LiteralPath $bodyFile -Value $body -Encoding utf8
    Remove-Item -LiteralPath $resFile -Force -ErrorAction SilentlyContinue
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $runner `
        -JobBodyFile $bodyFile -OutFile $resFile -TimeoutSeconds 900
    if ($LASTEXITCODE -ne 0) {
        throw "cred-inject job runner failed (exit $LASTEXITCODE)"
    }
    foreach ($l in (Get-Content -LiteralPath $resFile -Encoding UTF8)) {
        $lines.Add($l)
    }

    $evDir = 'D:\CLI\_windows_high_nist\formal-2026-09-02\evidence-cred-inject'
    New-Item -ItemType Directory -Path $evDir -Force | Out-Null
    $sess2 = New-PSSession -VMName $vmName -Credential $cred -ErrorAction Stop
    try {
        $guestFiles = @(Invoke-Command -Session $sess2 -ScriptBlock {
            Get-ChildItem -LiteralPath 'C:\workspace\cred-inject' -Filter '*.json' -File |
                ForEach-Object { $_.FullName }
        })
        foreach ($gf in $guestFiles) {
            Copy-Item -Path $gf -Destination $evDir -FromSession $sess2 -Force
            $lines.Add("EVIDENCE $gf")
        }
    }
    finally {
        Remove-PSSession -Session $sess2 -ErrorAction SilentlyContinue
    }

    $doneOk = @($lines | Where-Object { $_ -match '^CRED_INJECT_DONE$' }).Count -gt 0
    $errOk = @($lines | Where-Object { $_ -match '^CRED_INJECT_ERRORS=0$' }).Count -gt 0
    $ok = $doneOk -and $errOk
    $lines.Add("CRED_INJECT_OK=$ok")
}
catch {
    $lines.Add("ERROR: $($_.Exception.Message)")
    $lines.Add('CRED_INJECT_OK=False')
}

$lines | Set-Content -LiteralPath $OutFile -Encoding utf8
Write-Output "resultFile=$OutFile"
if (@($lines | Where-Object { $_ -match '^CRED_INJECT_OK=True$' }).Count -gt 0) {
    exit 0
}
exit 1
