<#
    run_agent_arm.ps1 - guest-side model-agent (orz) runner (P0-0l-7 agent k=1).

    Drives the real orz agent loop through the arm's native Windows wall for
    every task in the manifest (or -TaskIds), with the AppContainer
    credential-injection chain wired in (S4_PROGRESS_2026-09-02.md sec 16):

      1. BEFORE the first sandbox spawn, bootstrap the real DeepSeek key:
         a non-admin (AgentUser, non-AppContainer) sandbox runs the native
         cred_probe.exe with CRED_PROBE_DUMP and writes the raw CredReadW
         blob; the runner canonicalizes it in memory (never on argv, never
         printed).  Fail-closed: no key, no agent run.
      2. control   - ORZ_DEEPSEEK_API_KEY + ORZ_MAIN_AGENT_MODEL are set in
                     the runner process environment; the control-arm child
                     (current token, Job-contained) inherits it.
      3. high-nist - each orz spawn gets a per-run BOM-less env JSON injected
                     with --env-file (the sandbox CLI deletes it after a
                     successful read) and the sandbox is given
                     --allowlist-ip 221.204.163.76 (block rules win over
                     allow rules, so the process must be allowlisted before
                     it can reach the DeepSeek endpoint).  Since 2026-09-03
                     the high-nist agent arm also passes --no-appcontainer:
                     orz.exe fails DLL init (0xC0000142) as an AppContainer
                     process on Windows 11 25H2; the rest of the wall
                     (non-admin + LOW IL + Job + TEMP redirect + egress
                     allowlist) is unchanged.
      4. After the batch, every file that held the key is deleted and the
         runner verifies SECRET_FILES_LEFT=0.

    Evidence written under <Workspace>:
      agent-baseline-<arm>.json            batch summary
      cred-bootstrap-<arm>.json            secret-free bootstrap result
      run-observation-agent-<id>.json      sandbox run observation
      journal-agent-<id>.jsonl             orz events journal (events.jsonl)
      obs-cred-bootstrap.json              bootstrap run observation

    -DryRun prints the exact planned bootstrap/env/allowlist/command wiring
    without spawning anything or touching the credential store.
    ASCII only.
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidateSet('control', 'high-nist')]
    [string]$Arm,

    [string]$Workspace = '',

    [string]$TasksDir = 'C:\s4\_windows_high_nist\tasks',

    [string]$TaskIds = '',

    [string]$Python = 'C:\Program Files\Python312\python.exe',

    [string]$SandboxCli = 'C:\s4\scripts\run_windows_native_sandbox_command.py',

    [string]$CredExe = 'C:\s4\tools\cred_probe.exe',

    [string]$Orz = 'C:\Program Files\orz\orz.exe',

    [string]$AppRoot = 'C:\app',

    [string]$Model = 'deepseek-v4-flash',

    [int]$TaskTimeoutSeconds = 960,

    [int]$BootstrapTimeoutSeconds = 240,

    [string]$AllowlistIp = '221.204.163.76',

    [string]$ResultPath = '',

    [switch]$DryRun
)

$ErrorActionPreference = 'Continue'
$utf8NoBom = New-Object System.Text.UTF8Encoding($false)

if (-not $Workspace) {
    $wsKey = if ($Arm -eq 'high-nist') { 'agenthighnist' } else { 'agentcontrol' }
    $Workspace = Join-Path 'C:\workspace' $wsKey
}
New-Item -ItemType Directory -Path $Workspace -Force | Out-Null
if (-not $ResultPath) {
    $ResultPath = Join-Path $Workspace "agent-baseline-$Arm.json"
}

$summary = [ordered]@{
    schema_version = '0.1.0'
    arm = $Arm
    batch = ''
    model = $Model
    dry_run = [bool]$DryRun
    allowlist_ip = $AllowlistIp
    task_timeout_seconds = $TaskTimeoutSeconds
    bootstrap_ok = $false
    secret_files_left = -1
    tasks = New-Object System.Collections.ArrayList
    errors = New-Object System.Collections.ArrayList
}

$errors = New-Object System.Collections.ArrayList
$secretFiles = New-Object System.Collections.ArrayList

function Write-OutputLine([string]$Text) {
    Write-Output $Text
}

function Add-SecretFile([string]$Path) {
    if ($Path -and (Test-Path -LiteralPath $Path)) {
        [void]$secretFiles.Add($Path)
    }
}

# F2 (2026-09-03): C:\app is a per-task directory JUNCTION to the task
# workdir, so unmodified instructions that reference /app keep working and
# every /app read/write lands in the workdir (which the high-nist LOW-IL
# child can actually write — a real C:\app directory was integrity-denied).
function Reset-AppJunction {
    param([string]$Target)
    if ($DryRun) {
        Write-OutputLine "AGENT_DRYRUN app_junction C:\app -> $Target"
        return
    }
    try {
        if (Test-Path -LiteralPath $AppRoot) {
            Get-ChildItem -LiteralPath $AppRoot -Force -ErrorAction SilentlyContinue |
                Remove-Item -Recurse -Force -ErrorAction SilentlyContinue
            & cmd.exe /d /c ('rmdir "' + $AppRoot + '"') 2>&1 | Out-Null
        }
        & cmd.exe /d /c ('mklink /J "' + $AppRoot + '" "' + $Target + '"') 2>&1 | Out-Null
        $linkOk = (Test-Path -LiteralPath $AppRoot)
        Write-OutputLine ("AGENT_APP_JUNCTION ok=$linkOk target=" + $Target)
    }
    catch {
        Write-OutputLine "AGENT_APP_JUNCTION_ERR: $($_.Exception.Message)"
    }
}

function New-ProbeWorkspace {
    param([string]$Parent, [string]$Name)
    $wd = Join-Path $Parent $Name
    New-Item -ItemType Directory -Path $wd -Force | Out-Null
    $wdExe = Join-Path $wd 'cred_probe.exe'
    if (-not (Test-Path -LiteralPath $wdExe)) {
        Copy-Item -LiteralPath $CredExe -Destination $wdExe -Force
    }
    if (-not (Test-Path -LiteralPath $wdExe)) {
        throw "cred_probe.exe missing in $wd"
    }
    $marker = Join-Path $wd '.assurance-p2-disposable.json'
    if (-not (Test-Path -LiteralPath $marker)) {
        @{
            schema_version = '0.1.0-draft'
            purpose = 'windows-native-sandbox-probe'
            allow_container_write_probe = $true
        } | ConvertTo-Json | Set-Content -LiteralPath $marker -Encoding ascii
    }
    return $wd
}

# Non-admin (AgentUser, non-AppContainer) CredReadW bootstrap.  Returns the
# canonical key string; deletes the raw dump file immediately.  Secret never
# appears on argv or in any output line.
function Invoke-CredBootstrap {
    param([string]$BootstrapRoot)
    if ($DryRun) {
        Write-Output "AGENT_DRYRUN BOOTSTRAP: python $SandboxCli --workspace <cred-bootstrap-dir> --arm non-admin --env CRED_PROBE_DUMP=<dir>/key.blob --command <dir>/cred_probe.exe <readRes>"
        $script:AgentBootKey = ''
        return
    }
    $wd = New-ProbeWorkspace -Parent $BootstrapRoot -Name ('cred-bootstrap-' + [guid]::NewGuid().ToString('N'))
    $dumpPath = Join-Path $wd 'key.blob'
    $readRes = Join-Path $wd 'cred-read.json'
    $obsPath = Join-Path $BootstrapRoot 'obs-cred-bootstrap.json'
    Add-SecretFile $dumpPath

    $sbxArgs = @(
        '--workspace', $wd,
        '--arm', 'non-admin',
        '--timeout', "$BootstrapTimeoutSeconds",
        '--output', $obsPath,
        '--env', "CRED_PROBE_DUMP=$dumpPath",
        '--command',
        (Join-Path $wd 'cred_probe.exe'), $readRes
    )
    & $Python $SandboxCli @sbxArgs 2>&1 | ForEach-Object { Write-Output "AGENT_BOOT: $_" }
    $bootExit = $LASTEXITCODE
    Write-Output "AGENT_BOOT_EXIT=$bootExit"
    if (-not (Test-Path -LiteralPath $readRes)) {
        throw "bootstrap cred-read.json missing (exit=$bootExit)"
    }
    $r = Get-Content -LiteralPath $readRes -Raw -Encoding UTF8 | ConvertFrom-Json
    Write-Output ("AGENT_BOOT ok=" + $r.ok + " winerror=" + $r.winerror +
        " persist=" + $r.persist + " blob_bytes=" + $r.blob_bytes +
        " dump_ok=" + $r.dump_ok)
    if (-not ($r.ok -and $r.dump_ok -and [int]$r.blob_bytes -gt 0)) {
        throw 'bootstrap CredReadW/dump incomplete (fail-closed)'
    }
    if (-not (Test-Path -LiteralPath $dumpPath)) {
        throw 'bootstrap dump file missing (fail-closed)'
    }

    # Canonicalize: the CredReadW blob is UTF-16-LE text with a trailing NUL.
    $raw = [System.IO.File]::ReadAllBytes($dumpPath)
    $keyText = [System.Text.Encoding]::Unicode.GetString($raw)
    $keyText = $keyText.TrimEnd([char]0)
    if ([string]::IsNullOrWhiteSpace($keyText) -or $keyText.Length -lt 20) {
        throw 'bootstrap key canonicalization produced an empty/short key (fail-closed)'
    }

    # Evidence copy of the secret-free probe result, then delete every file
    # that held key bytes.
    Copy-Item -LiteralPath $readRes -Destination (Join-Path $BootstrapRoot "cred-bootstrap-$Arm.json") -Force
    Remove-Item -LiteralPath $dumpPath -Force -ErrorAction SilentlyContinue
    $summary.bootstrap_ok = $true
    Write-Output ("AGENT_CRED_BOOTSTRAP_OK=True key_chars=" + $keyText.Length)
    # PowerShell captures every success-stream write of a function into the
    # caller's assignment; hand the canonical key back via script scope so
    # the console/diagnostic lines above never contaminate the env value.
    $script:AgentBootKey = $keyText
}

function Write-SandboxEnvJson {
    param([string]$Path, [string]$Key)
    if ($Key -notmatch '^sk-[A-Za-z0-9]{32}$') {
        throw 'env-file key failed canonical shape validation (fail-closed)'
    }
    $envObj = [ordered]@{
        ORZ_DEEPSEEK_API_KEY = $Key
        ORZ_MAIN_AGENT_MODEL = $Model
    }
    [System.IO.File]::WriteAllText($Path, ($envObj | ConvertTo-Json -Compress), $utf8NoBom)
}

# ---- manifest / task selection --------------------------------------------
$manifest = Join-Path $TasksDir 'manifest.json'
if (-not (Test-Path -LiteralPath $manifest)) {
    Write-Output "AGENT_ERROR manifest missing: $manifest"
    exit 1
}
$m = Get-Content -LiteralPath $manifest -Raw -Encoding UTF8 | ConvertFrom-Json
$batch = if ($m.batch) { [string]$m.batch } else { 'P0-0l-agent' }
$summary.batch = $batch
$allIds = @($m.tasks | ForEach-Object { $_.id })

$selectedIds = @()
if ([string]::IsNullOrWhiteSpace($TaskIds)) {
    $selectedIds = @($allIds)
}
else {
    foreach ($rawId in ($TaskIds -split ',')) {
        $tid = $rawId.Trim()
        if ($tid) {
            if ($allIds -notcontains $tid) {
                Write-Output "AGENT_ERROR unknown task id: $tid"
                [void]$errors.Add("unknown task id: $tid")
            }
            else {
                $selectedIds += $tid
            }
        }
    }
    if ($selectedIds.Count -eq 0) {
        Write-Output 'AGENT_ERROR no valid task ids after filtering'
        exit 1
    }
}
Write-Output "AGENT_MANIFEST batch=$batch tasks=$($selectedIds -join ',')"

# ---- bootstrap (once, before any target-arm spawn) ------------------------
$keyText = ''
try {
    $bootLines = @(Invoke-CredBootstrap -BootstrapRoot $Workspace)
    foreach ($bootLine in $bootLines) {
        if ($bootLine -is [string] -and $bootLine) {
            Write-Output $bootLine
        }
    }
    $keyText = [string]$script:AgentBootKey
}
catch {
    [void]$errors.Add("credential bootstrap failed: $($_.Exception.Message)")
    [void]$summary.errors.Add("credential bootstrap failed: $($_.Exception.Message)")
    Write-Output "AGENT_CRED_BOOTSTRAP_FAILED: $($_.Exception.Message)"
}

if ($DryRun) {
    $injectLabel = if ($Arm -eq 'high-nist') { 'env-file' } else { 'process-env' }
    Write-Output "AGENT_DRYRUN env_channel=ORZ_DEEPSEEK_API_KEY inject=$injectLabel allowlist=$AllowlistIp"
    Write-Output "AGENT_DRYRUN orz=$Orz model=$Model arm=$Arm"
}

foreach ($taskId in $selectedIds) {
    if ($errors.Count -gt 0) {
        Write-Output "AGENT_ABORT_SKIP_TASK task=$taskId (prior errors)"
        continue
    }
    $taskDir = Join-Path $TasksDir $taskId
    $taskJsonPath = Join-Path $taskDir 'task.json'
    $instructionPath = Join-Path $taskDir 'instruction.md'
    if (-not (Test-Path -LiteralPath $taskJsonPath) -or -not (Test-Path -LiteralPath $instructionPath)) {
        Write-Output "AGENT_ERROR task $taskId missing task.json/instruction.md"
        [void]$errors.Add("task $taskId missing task.json/instruction.md")
        continue
    }
    $tj = Get-Content -LiteralPath $taskJsonPath -Raw -Encoding UTF8 | ConvertFrom-Json
    $perTaskTimeoutSeconds = $TaskTimeoutSeconds
    if ($tj.agent_timeout_seconds) {
        try {
            $perTaskTimeoutSeconds = [int]$tj.agent_timeout_seconds
        }
        catch {
            $perTaskTimeoutSeconds = $TaskTimeoutSeconds
        }
    }
    if ($perTaskTimeoutSeconds -lt 60) {
        $perTaskTimeoutSeconds = 60
    }
    # TER T2.1 (2026-09-04) 墙钟单一化：不再派生 `perTask-60` 的
    # `--max-wallclock`（840 余量删除）——sandbox `--timeout`（=官方
    # agent_timeout_seconds）是唯一评测墙钟；官方值经 `ORZ_MAX_WALLCLOCK`
    # env 透传给 orz 内部（F6 pull/push 读源以 runner 施加值为准，
    # T1.8/T1.9 已消费该 env）。
    $workdir = Join-Path $Workspace ("task-" + $taskId)
    New-Item -ItemType Directory -Path $workdir -Force | Out-Null
    Copy-Item -LiteralPath $instructionPath -Destination $workdir -Force
    Copy-Item -LiteralPath $taskJsonPath -Destination $workdir -Force

    # F2 (2026-09-03): task input pack (taskDir\inputs + inputs-manifest.json)
    # is copied into the workdir and mirrored into C:\app before the spawn.
    $inputsDir = Join-Path $taskDir 'inputs'
    $inputsManifest = Join-Path $taskDir 'inputs-manifest.json'
    if (Test-Path -LiteralPath $inputsDir) {
        if (-not $DryRun) {
            Copy-Item -LiteralPath (Join-Path $inputsDir '*') -Destination $workdir -Recurse -Force
        }
        Write-OutputLine "AGENT_INPUTS_LOADED task=$taskId count=$(@(Get-ChildItem -LiteralPath $inputsDir -Recurse -File).Count)"
    }
    else {
        Write-OutputLine "AGENT_INPUTS_LOADED task=$taskId count=0 (no-input task)"
    }
    if ((-not $DryRun) -and (Test-Path -LiteralPath $inputsManifest)) {
        Copy-Item -LiteralPath $inputsManifest -Destination (Join-Path $workdir 'inputs-manifest.json') -Force
    }
    Reset-AppJunction -Target $workdir

    $marker = Join-Path $workdir '.assurance-p2-disposable.json'
    if (-not (Test-Path -LiteralPath $marker)) {
        @{
            schema_version = '0.1.0-draft'
            purpose = 'windows-native-sandbox-probe'
            allow_container_write_probe = $true
        } | ConvertTo-Json | Set-Content -LiteralPath $marker -Encoding ascii
    }

    # Deterministic environment seed (same as the machine-side task runner:
    # executed host-side before the sandbox spawn).
    if ($tj.environment_seed) {
        $seed = Join-Path $taskDir ('environment\' + $tj.environment_seed)
        if (Test-Path -LiteralPath $seed) {
            Copy-Item -LiteralPath $seed -Destination $workdir -Force
            Push-Location $workdir
            try {
                & $Python (Join-Path $workdir $tj.environment_seed) 2>&1 |
                    ForEach-Object { Write-Output "AGENT_SEED: $_" }
            }
            finally {
                Pop-Location
            }
        }
    }

    $prompt = (Get-Content -LiteralPath $instructionPath -Raw -Encoding UTF8).Trim()
    Write-Output "===== AGENT TASK $taskId (arm=$Arm) ====="

    $obsPath = Join-Path $workdir 'agent-run-observation.json'
    $envFilePath = ''
    if ($Arm -eq 'high-nist' -and -not $DryRun) {
        $envFilePath = Join-Path $Workspace ("agent-env-" + $taskId + '-' + [guid]::NewGuid().ToString('N') + '.json')
        Add-SecretFile $envFilePath
        Write-SandboxEnvJson -Path $envFilePath -Key $keyText
    }

    $sbxArgs = @(
        '--workspace', $workdir,
        '--arm', $Arm,
        '--timeout', "$perTaskTimeoutSeconds",
        '--output', $obsPath
    )
    if ($Arm -eq 'high-nist') {
        if ($DryRun) {
            $sbxArgs += @('--env-file', '<agent-env-json>')
        }
        else {
            $sbxArgs += @('--env-file', $envFilePath)
        }
        $sbxArgs += @('--no-appcontainer')
        if (-not [string]::IsNullOrWhiteSpace($AllowlistIp)) {
            $sbxArgs += @('--allowlist-ip', $AllowlistIp)
        }
    }
    $sbxArgs += @(
        '--command',
        $Orz,
        '-p', $prompt,
        '--real',
        '--allow-write',
        '--allow-shell',
        '--allow-network',
        '--run-root', $workdir
    )
    $env:ORZ_MAX_WALLCLOCK = "$perTaskTimeoutSeconds"

    if ($Arm -eq 'control') {
        if (-not $DryRun) {
            $env:ORZ_DEEPSEEK_API_KEY = $keyText
            $env:ORZ_MAIN_AGENT_MODEL = $Model
        }
    }

    $taskResult = [ordered]@{
        id = $taskId
        attempt = 'ran'
        orz_exit = -1
        observation_outcome = ''
        journal_copied = $false
        agent_timeout_seconds = $perTaskTimeoutSeconds
        env_inject = $(if ($Arm -eq 'high-nist') { 'env-file' } else { 'process-env' })
        allowlist = $(if ($Arm -eq 'high-nist') { $AllowlistIp } else { '' })
        error = ''
    }

    if ($DryRun) {
        Write-Output "AGENT_DRYRUN TASK=$taskId env=$($taskResult.env_inject) allowlist=$($taskResult.allowlist) appcontainer=off"
        Write-Output "AGENT_DRYRUN SBX: python $SandboxCli $($sbxArgs -join ' ')"
        Write-Output "AGENT_DRYRUN wallclock runner_imposed=sandbox --timeout $perTaskTimeoutSeconds max_wallclock_arg=none env_ORZ_MAX_WALLCLOCK=$perTaskTimeoutSeconds"
        $taskResult.attempt = 'dry-run'
        $taskResult.orz_exit = 0
        [void]$summary.tasks.Add($taskResult)
        continue
    }

    & $Python $SandboxCli @sbxArgs 2>&1 | ForEach-Object { Write-Output "AGENT_RUN: $_" }
    $runExit = $LASTEXITCODE
    Write-Output "AGENT_RUN_EXIT=$runExit"

    if (-not (Test-Path -LiteralPath $obsPath)) {
        $taskResult.attempt = 'spawn-failed'
        $taskResult.error = "run observation missing (runExit=$runExit)"
        [void]$errors.Add("task $taskId run observation missing (runExit=$runExit)")
        [void]$summary.tasks.Add($taskResult)
        continue
    }
    $o = Get-Content -LiteralPath $obsPath -Raw -Encoding UTF8 | ConvertFrom-Json
    $taskResult.observation_outcome = [string]$o.outcome
    $taskResult.orz_exit = [int64]$o.process.exit_code
    if ($o.outcome -ne 'compliant') {
        $taskResult.attempt = 'wall-noncompliant'
        $taskResult.error = 'sandbox observation outcome=' + [string]$o.outcome
        [void]$errors.Add("task $taskId observation outcome=" + [string]$o.outcome)
    }
    elseif ([int64]$o.process.exit_code -ne 0) {
        $taskResult.attempt = 'error'
        $taskResult.error = 'orz exit code=' + [string]$o.process.exit_code
    }

    # orz journal (events.jsonl under <workdir>\.gsa\...) - the raw material
    # for design sec 7 event-face criteria (err/slow/stall/LIF/journal).
    $journal = Get-ChildItem -LiteralPath $workdir -Recurse -File -Filter 'events.jsonl' -ErrorAction SilentlyContinue |
        Sort-Object LastWriteTime -Descending | Select-Object -First 1
    if ($journal) {
        Copy-Item -LiteralPath $journal.FullName -Destination (Join-Path $Workspace ("journal-agent-" + $taskId + '.jsonl')) -Force
        $taskResult.journal_copied = $true
    }
    Copy-Item -LiteralPath $obsPath -Destination (Join-Path $Workspace ("run-observation-agent-" + $taskId + '.json')) -Force
    [void]$summary.tasks.Add($taskResult)
}

# ---- cleanup ----------------------------------------------------------------
if (-not $DryRun) {
    Remove-Item Env:ORZ_DEEPSEEK_API_KEY -ErrorAction SilentlyContinue
    Remove-Item Env:ORZ_MAIN_AGENT_MODEL -ErrorAction SilentlyContinue
    Remove-Item Env:ORZ_MAX_WALLCLOCK -ErrorAction SilentlyContinue
}
foreach ($f in @($secretFiles)) {
    if (Test-Path -LiteralPath $f) {
        Remove-Item -LiteralPath $f -Force -ErrorAction SilentlyContinue
    }
}
$left = @()
if (-not $DryRun) {
    $left += @(Get-ChildItem -LiteralPath $Workspace -Recurse -File -ErrorAction SilentlyContinue |
        Where-Object { $_.Name -match '^agent-env-.*\.json$|^key\.blob$|^inject-env.*\.json$|^key-canon.*\.txt$' } |
        Select-Object -ExpandProperty FullName)
}
Write-Output "SECRET_FILES_LEFT=$($left.Count)"
if ($left.Count -gt 0) {
    foreach ($f in $left) { Write-Output "SECRET_LEFT: $f" }
    [void]$errors.Add('secret cleanup incomplete')
}
$summary.secret_files_left = $left.Count
$summary.errors = $errors

[System.IO.File]::WriteAllText(
    $ResultPath,
    ($summary | ConvertTo-Json -Depth 8),
    $utf8NoBom
)
Write-Output "AGENT_SUMMARY=$ResultPath"
Write-Output "AGENT_ERRORS=$($errors.Count)"
foreach ($e in $errors) { Write-Output "AGENT_ERROR: $e" }
Write-Output 'AGENT_DONE'
if ($errors.Count -gt 0) {
    exit 1
}
exit 0
