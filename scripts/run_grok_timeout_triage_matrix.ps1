[CmdletBinding()]
param(
    [string]$RunId = ("grok-timeout-triage-" + (Get-Date -Format "yyyyMMdd-HHmmss")),
    [string]$PythonPath = "python",
    [int]$TimeoutSeconds = 60,
    [int]$ToolTimeoutMilliseconds = 3000,
    [int]$ExitGraceSeconds = 10
)

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..")).Path
$runRoot = Join-Path $repoRoot (Join-Path "candidate-gates" $RunId)
New-Item -ItemType Directory -Path $runRoot -Force | Out-Null

function Write-JsonFile {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)]$Value
    )
    $encoding = New-Object System.Text.UTF8Encoding($false)
    [System.IO.File]::WriteAllText($Path, ($Value | ConvertTo-Json -Depth 14), $encoding)
}

function Invoke-TimeoutProbe {
    param(
        [Parameter(Mandatory = $true)][string]$Version,
        [Parameter(Mandatory = $true)][string]$MetadataPath,
        [Parameter(Mandatory = $true)][string]$BinaryPath
    )
    $outputDirectory = Join-Path $runRoot "$Version-tool-timeout"
    $stdoutPath = Join-Path $runRoot "$Version-tool-timeout.stdout.txt"
    $stderrPath = Join-Path $runRoot "$Version-tool-timeout.stderr.txt"
    $scriptPath = Join-Path $repoRoot "scripts\invoke_grok_windows_child_tree_probe.ps1"
    $startedAt = (Get-Date).ToUniversalTime().ToString("o")
    $previousErrorActionPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        $output = & powershell -NoProfile -ExecutionPolicy Bypass -File $scriptPath `
            -OutputDirectory $outputDirectory `
            -Scenario tool_timeout `
            -ReleaseMetadataPath $MetadataPath `
            -BinaryPath $BinaryPath `
            -PythonPath $PythonPath `
            -TimeoutSeconds $TimeoutSeconds `
            -ToolTimeoutMilliseconds $ToolTimeoutMilliseconds `
            -ExitGraceSeconds $ExitGraceSeconds 2>&1
        $exitCode = $LASTEXITCODE
    } catch {
        $output = @($_)
        $exitCode = 1
    } finally {
        $ErrorActionPreference = $previousErrorActionPreference
    }
    $finishedAt = (Get-Date).ToUniversalTime().ToString("o")
    $encoding = New-Object System.Text.UTF8Encoding($false)
    [System.IO.File]::WriteAllText($stdoutPath, ($output | Out-String), $encoding)
    if ($exitCode -eq 0) {
        [System.IO.File]::WriteAllText($stderrPath, "", $encoding)
    } else {
        [System.IO.File]::WriteAllText($stderrPath, ($output | Out-String), $encoding)
    }
    return [ordered]@{
        version = $Version
        output_directory = $outputDirectory
        stdout = $stdoutPath
        stderr = $stderrPath
        started_at = $startedAt
        finished_at = $finishedAt
        exit_code = [int]$exitCode
        passed = ($exitCode -eq 0)
        result_exists = (Test-Path -LiteralPath (Join-Path $outputDirectory "result.json"))
        verification_exists = (Test-Path -LiteralPath (Join-Path $outputDirectory "verification.json"))
        failure_exists = (Test-Path -LiteralPath (Join-Path $outputDirectory "failure.json"))
        timeout_diagnostic_exists = (Test-Path -LiteralPath (Join-Path $outputDirectory "timeout-diagnostic.json"))
    }
}

$matrix = @(
    [ordered]@{
        version = "0.2.111"
        metadata = Join-Path $repoRoot "upstream\grok-build.lock.json"
        binary = Join-Path $repoRoot ".tools\grok\0.2.111\grok.exe"
    },
    [ordered]@{
        version = "0.2.112"
        metadata = Join-Path $repoRoot "upstream\grok-build.candidate.json"
        binary = Join-Path $repoRoot ".tools\grok\0.2.112\grok.exe"
    }
)

$results = @()
foreach ($entry in $matrix) {
    $results += Invoke-TimeoutProbe -Version $entry.version -MetadataPath $entry.metadata -BinaryPath $entry.binary
}

$summary = [ordered]@{
    schema_version = "0.1.0"
    receipt_kind = "grok-timeout-triage-matrix"
    run_id = $RunId
    run_root = $runRoot
    created_at = (Get-Date).ToUniversalTime().ToString("o")
    timeout_seconds = $TimeoutSeconds
    tool_timeout_ms = $ToolTimeoutMilliseconds
    exit_grace_seconds = $ExitGraceSeconds
    all_passed = (@($results | Where-Object { -not $_.passed }).Count -eq 0)
    results = $results
}
$summaryPath = Join-Path $runRoot "summary.json"
Write-JsonFile -Path $summaryPath -Value $summary
$summary | ConvertTo-Json -Depth 14
if ($summary.all_passed) { exit 0 }
exit 1
