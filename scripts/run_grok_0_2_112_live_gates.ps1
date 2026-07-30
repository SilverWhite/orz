[CmdletBinding()]
param(
    [string]$RunId = ("grok-0.2.112-live-gates-" + (Get-Date -Format "yyyyMMdd-HHmmss")),
    [string]$PythonPath = "python"
)

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..")).Path
$candidateMetadata = Join-Path $repoRoot "upstream\grok-build.candidate.json"
$runRoot = Join-Path $repoRoot (Join-Path "candidate-gates" $RunId)
New-Item -ItemType Directory -Path $runRoot -Force | Out-Null

function Write-JsonFile {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)]$Value
    )
    $encoding = New-Object System.Text.UTF8Encoding($false)
    [System.IO.File]::WriteAllText($Path, ($Value | ConvertTo-Json -Depth 12), $encoding)
}

function Invoke-ProbeStep {
    param(
        [Parameter(Mandatory = $true)][string]$Id,
        [Parameter(Mandatory = $true)][string]$ScriptPath,
        [Parameter(Mandatory = $true)][string[]]$Arguments
    )
    $stepRoot = Join-Path $runRoot $Id
    New-Item -ItemType Directory -Path $stepRoot -Force | Out-Null
    $stdoutPath = Join-Path $stepRoot "stdout.txt"
    $stderrPath = Join-Path $stepRoot "stderr.txt"
    $outputIndex = [Array]::IndexOf($Arguments, "-OutputDirectory")
    if ($outputIndex -ge 0 -and ($outputIndex + 1) -lt $Arguments.Count) {
        $probeOutputDirectory = $Arguments[$outputIndex + 1]
    } else {
        $probeOutputDirectory = $stepRoot
    }

    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = "powershell"
    $nativeArguments = @("-NoProfile", "-ExecutionPolicy", "Bypass", "-File", $ScriptPath) + $Arguments
    $psi.Arguments = (($nativeArguments | ForEach-Object {
        if ($_.Contains('"')) {
            throw "Native argument may not contain a quote character: $_"
        }
        '"' + $_ + '"'
    }) -join " ")
    $psi.WorkingDirectory = $repoRoot
    $psi.UseShellExecute = $false
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true

    $startedAt = (Get-Date).ToUniversalTime().ToString("o")
    $process = [System.Diagnostics.Process]::Start($psi)
    $stdout = $process.StandardOutput.ReadToEnd()
    $stderr = $process.StandardError.ReadToEnd()
    $process.WaitForExit()
    $finishedAt = (Get-Date).ToUniversalTime().ToString("o")

    $encoding = New-Object System.Text.UTF8Encoding($false)
    [System.IO.File]::WriteAllText($stdoutPath, $stdout, $encoding)
    [System.IO.File]::WriteAllText($stderrPath, $stderr, $encoding)

    return [ordered]@{
        id = $Id
        script = $ScriptPath
        output_directory = $probeOutputDirectory
        wrapper_output_directory = $stepRoot
        started_at = $startedAt
        finished_at = $finishedAt
        exit_code = [int]$process.ExitCode
        passed = ($process.ExitCode -eq 0)
        stdout = $stdoutPath
        stderr = $stderrPath
    }
}

$steps = @(
    [ordered]@{
        id = "acp_initialize"
        script = Join-Path $repoRoot "scripts\invoke_grok_acp_initialize_probe.ps1"
        args = @("-OutputDirectory", (Join-Path $runRoot "acp-initialize"), "-TimeoutSeconds", "30", "-PythonPath", $PythonPath, "-ReleaseMetadataPath", $candidateMetadata)
    },
    [ordered]@{
        id = "fake_tool_allow"
        script = Join-Path $repoRoot "scripts\invoke_grok_acp_fake_tool_probe.ps1"
        args = @("-OutputDirectory", (Join-Path $runRoot "fake-tool-allow"), "-Scenario", "allow_once", "-TimeoutSeconds", "45", "-PythonPath", $PythonPath, "-ReleaseMetadataPath", $candidateMetadata)
    },
    [ordered]@{
        id = "fake_tool_cancel"
        script = Join-Path $repoRoot "scripts\invoke_grok_acp_fake_tool_probe.ps1"
        args = @("-OutputDirectory", (Join-Path $runRoot "fake-tool-cancel"), "-Scenario", "cancel_permission", "-TimeoutSeconds", "45", "-PythonPath", $PythonPath, "-ReleaseMetadataPath", $candidateMetadata)
    },
    [ordered]@{
        id = "windows_child_tree_timeout"
        script = Join-Path $repoRoot "scripts\invoke_grok_windows_child_tree_probe.ps1"
        args = @("-OutputDirectory", (Join-Path $runRoot "child-tree-timeout"), "-Scenario", "tool_timeout", "-ReleaseMetadataPath", $candidateMetadata, "-PythonPath", $PythonPath, "-TimeoutSeconds", "180")
    },
    [ordered]@{
        id = "windows_child_tree_task_cancel"
        script = Join-Path $repoRoot "scripts\invoke_grok_windows_child_tree_probe.ps1"
        args = @("-OutputDirectory", (Join-Path $runRoot "child-tree-task-cancel"), "-Scenario", "task_cancel", "-ReleaseMetadataPath", $candidateMetadata, "-PythonPath", $PythonPath, "-TimeoutSeconds", "60")
    },
    [ordered]@{
        id = "windows_child_tree_parent_exit"
        script = Join-Path $repoRoot "scripts\invoke_grok_windows_child_tree_probe.ps1"
        args = @("-OutputDirectory", (Join-Path $runRoot "child-tree-parent-exit"), "-Scenario", "parent_exit", "-ReleaseMetadataPath", $candidateMetadata, "-PythonPath", $PythonPath, "-TimeoutSeconds", "60")
    },
    [ordered]@{
        id = "compaction_provenance"
        script = Join-Path $repoRoot "scripts\invoke_grok_compaction_provenance_probe.ps1"
        args = @("-OutputDirectory", (Join-Path $runRoot "compaction-provenance"), "-ReleaseMetadataPath", $candidateMetadata, "-PythonPath", $PythonPath, "-TimeoutSeconds", "90")
    }
)

$results = @()
foreach ($step in $steps) {
    $result = Invoke-ProbeStep -Id $step.id -ScriptPath $step.script -Arguments $step.args
    $results += $result
}

$summary = [ordered]@{
    schema_version = "0.1.0"
    receipt_kind = "grok-0.2.112-live-gates-summary"
    run_id = $RunId
    run_root = $runRoot
    created_at = (Get-Date).ToUniversalTime().ToString("o")
    candidate_metadata = $candidateMetadata
    all_passed = (@($results | Where-Object { -not $_.passed }).Count -eq 0)
    results = $results
}
$summaryPath = Join-Path $runRoot "summary.json"
Write-JsonFile -Path $summaryPath -Value $summary
$summary | ConvertTo-Json -Depth 12
if ($summary.all_passed) { exit 0 }
exit 1
