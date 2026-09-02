<#
    P0-0l task-set runner (guest-side, host-elevated session via driver).

    Usage:
      .\run_task_arm.ps1 -Arm control -Workspace C:\workspace\taskcontrol `
          -TasksDir C:\s4\_windows_high_nist\tasks -ResultPath <json>

    For each task in the manifest:
      1. copy task probe + task.json + _tasklib.py into its workdir;
      2. seed environment (log-summary-date-ranges) when configured;
      3. run the task action under the arm's native Windows wall via
         run_windows_native_sandbox_command.py (control = Job-contained);
      4. run verify_task.py natively and aggregate PASS/FAIL.

    Writes <Workspace>\task-baseline-<arm>.json; exit 0 only if every task
    verifier passed (fail-closed).  ASCII only.
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidateSet('control', 'non-admin', 'high-nist')]
    [string]$Arm,

    [string]$Workspace = 'C:\workspace\taskcontrol',

    [string]$TasksDir = 'C:\s4\_windows_high_nist\tasks',

    [string]$Python = 'C:\Program Files\Python312\python.exe',

    [string]$ResultPath = ''
)

$ErrorActionPreference = 'Continue'
$sandboxCli = 'C:\s4\scripts\run_windows_native_sandbox_command.py'
$verifyScript = Join-Path $TasksDir 'verify_task.py'
$manifest = Join-Path $TasksDir 'manifest.json'
if (-not $ResultPath) {
    $ResultPath = Join-Path $Workspace "task-baseline-$Arm.json"
}

New-Item -ItemType Directory -Path $Workspace -Force | Out-Null
$m = Get-Content -LiteralPath $manifest -Raw -Encoding UTF8 | ConvertFrom-Json
$taskIds = @($m.tasks | ForEach-Object { $_.id })
$batch = if ($m.batch) { [string]$m.batch } else { 'P0-0l-batch1' }
Write-Output "BATCH=$batch"

$results = New-Object System.Collections.Generic.List[object]
$anyFail = $false

foreach ($taskId in $taskIds) {
    $taskDir = Join-Path $TasksDir $taskId
    $taskJsonPath = Join-Path $taskDir 'task.json'
    $tj = Get-Content -LiteralPath $taskJsonPath -Raw -Encoding UTF8 | ConvertFrom-Json
    $workdir = Join-Path $Workspace ("task-" + $taskId)
    New-Item -ItemType Directory -Path $workdir -Force | Out-Null

    Write-Output "===== TASK $taskId (arm=$Arm) ====="

    # Copy probe assets into the workdir (AppContainer children can only
    # read paths granted to the package SID).
    Copy-Item -LiteralPath (Join-Path $TasksDir '_tasklib.py') -Destination $workdir -Force
    Copy-Item -LiteralPath (Join-Path $taskDir 'probe.py') -Destination $workdir -Force
    Copy-Item -LiteralPath $taskJsonPath -Destination $workdir -Force

    # Disposable-workspace marker required by windows_sandbox.py.
    $marker = Join-Path $workdir '.assurance-p2-disposable.json'
    if (-not (Test-Path -LiteralPath $marker)) {
        @{
            schema_version = '0.1.0-draft'
            purpose = 'windows-native-sandbox-probe'
            allow_container_write_probe = $true
        } | ConvertTo-Json | Set-Content -LiteralPath $marker -Encoding ascii
    }

    # Seed environment (e.g. deterministic log files).
    if ($tj.environment_seed) {
        $seed = Join-Path $taskDir ('environment\' + $tj.environment_seed)
        if (Test-Path -LiteralPath $seed) {
            Copy-Item -LiteralPath $seed -Destination $workdir -Force
            Push-Location $workdir
            try {
                & $Python (Join-Path $workdir $tj.environment_seed) 2>&1 |
                    ForEach-Object { Write-Output "SEED: $_" }
            }
            finally {
                Pop-Location
            }
        }
    }

    # Run the task action under the arm's wall (same pattern as
    # run_enforcement_probe.ps1: --command must be the last option).
    $outcomePath = Join-Path $workdir 'outcome.json'
    $obsPath = Join-Path $workdir 'run-observation.json'
    Remove-Item -LiteralPath $outcomePath -Force -ErrorAction SilentlyContinue
    $probePath = Join-Path $workdir 'probe.py'
    $sbxTimeout = 240
    if ($tj.sandbox_timeout_seconds) {
        try {
            $sbxTimeout = [int]$tj.sandbox_timeout_seconds
        }
        catch {
            $sbxTimeout = 240
        }
    }
    $sbxArgs = @(
        '--workspace', $workdir,
        '--arm', $Arm,
        '--timeout', $sbxTimeout,
        '--output', $obsPath,
        '--command',
        'python.exe', $probePath,
        '-Arm', $Arm,
        '-Workdir', $workdir,
        '-OutcomePath', $outcomePath
    )
    & $Python $sandboxCli @sbxArgs 2>&1 |
        ForEach-Object { Write-Output "RUN: $_" }
    $runExit = $LASTEXITCODE
    Write-Output "TASK_RUN_EXIT=$runExit"

    $attempt = 'failed'
    $errText = ''
    $obsOutcome = ''
    if (Test-Path -LiteralPath $obsPath) {
        $o = Get-Content -LiteralPath $obsPath -Raw -Encoding UTF8 | ConvertFrom-Json
        $obsOutcome = $o.outcome
        $runExit = [int64]$o.process.exit_code
        Write-Output "OBS_OUTCOME=$obsOutcome OBS_EXIT=$runExit"
        if ($o.outcome -ne 'compliant') {
            $anyFail = $true
        }
    }
    else {
        $anyFail = $true
        $errText = 'run observation missing'
    }
    if (Test-Path -LiteralPath $outcomePath) {
        $o = Get-Content -LiteralPath $outcomePath -Raw -Encoding UTF8 | ConvertFrom-Json
        $attempt = $o.attempt
        $errText = $o.error
    }
    else {
        $anyFail = $true
        if (-not $errText) {
            $errText = 'outcome.json missing (probe did not finish)'
        }
    }

    # Verifier (native, trusted side).
    $verifyOut = Join-Path $Workspace ("verify-" + $taskId + '.json')
    $verifyExit = 1
    if (Test-Path -LiteralPath $outcomePath) {
        & $Python $verifyScript -Arm $Arm -TaskJson (Join-Path $workdir 'task.json') `
            -Workdir $workdir -OutcomePath $outcomePath -VerifyOut $verifyOut 2>&1 |
            ForEach-Object { Write-Output "VERIFY: $_" }
        $verifyExit = $LASTEXITCODE
        Write-Output "VERIFY_EXIT=$verifyExit"
        if ($verifyExit -ne 0) { $anyFail = $true }
    }
    else {
        Write-Output 'VERIFY_SKIPPED=outcome missing'
    }

    $obsCopy = Join-Path $Workspace ("run-observation-" + $taskId + '.json')
    if (Test-Path -LiteralPath $obsPath) {
        Copy-Item -LiteralPath $obsPath -Destination $obsCopy -Force
    }
    $outcomeCopy = Join-Path $Workspace ("task-outcome-" + $taskId + '.json')
    if (Test-Path -LiteralPath $outcomePath) {
        Copy-Item -LiteralPath $outcomePath -Destination $outcomeCopy -Force
    }

    $results.Add([ordered]@{
        id = $taskId
        attempt = $attempt
        error = $errText
        run_exit = $runExit
        observation_outcome = $obsOutcome
        verify_exit = $verifyExit
        passed = ($verifyExit -eq 0)
        verify_file = $verifyOut
    })
}

$summary = [ordered]@{
    schema_version = '0.1.0'
    arm = $Arm
    batch = $batch
    tasks = $results
    passed_all = (-not $anyFail)
    ran_at = (Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ')
}
$summary | ConvertTo-Json -Depth 6 |
    Set-Content -LiteralPath $ResultPath -Encoding utf8
Write-Output "SUMMARY=$ResultPath"
Write-Output "PASSED_ALL=$(-not $anyFail)"
if ($anyFail) {
    exit 1
}
exit 0
