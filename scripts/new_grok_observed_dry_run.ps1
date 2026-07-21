[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$PromptFile,

    [Parameter(Mandatory = $true)]
    [string]$ConfigPath,

    [Parameter(Mandatory = $true)]
    [string]$OutputDirectory,

    [Parameter(Mandatory = $true)]
    [string]$ModelAlias,

    [Parameter(Mandatory = $true)]
    [ValidateSet('high', 'max')]
    [string]$ReasoningEffort,

    [Parameter(Mandatory = $true)]
    [ValidateRange(1, 1000)]
    [int]$MaxTurns,

    [Parameter(Mandatory = $true)]
    [ValidateNotNullOrEmpty()]
    [string]$SandboxProfile,

    [Parameter(Mandatory = $true)]
    [ValidateSet('disabled', 'fake-loopback-only')]
    [string]$NetworkPolicy,

    [string[]]$ToolAllowlist = @(),

    [string]$BinaryPath
)

$ErrorActionPreference = 'Stop'
$repoRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path

function Get-FileRecord {
    param([Parameter(Mandatory = $true)][string]$Path)

    $resolved = (Resolve-Path -LiteralPath $Path).Path
    $item = Get-Item -LiteralPath $resolved
    if ($item.PSIsContainer) {
        throw "Expected a file, got a directory: $resolved"
    }
    return [ordered]@{
        path = $resolved
        bytes = $item.Length
        sha256 = (Get-FileHash -LiteralPath $resolved -Algorithm SHA256).Hash.ToLowerInvariant()
    }
}

function Get-TextSha256 {
    param([AllowEmptyString()][string]$Text)

    $algorithm = [System.Security.Cryptography.SHA256]::Create()
    try {
        $bytes = [System.Text.Encoding]::UTF8.GetBytes($Text)
        $hash = $algorithm.ComputeHash($bytes)
        return ([System.BitConverter]::ToString($hash)).Replace('-', '').ToLowerInvariant()
    } finally {
        $algorithm.Dispose()
    }
}

$prompt = Get-FileRecord -Path $PromptFile
$config = Get-FileRecord -Path $ConfigPath
$outputPath = [System.IO.Path]::GetFullPath($OutputDirectory)
if (Test-Path -LiteralPath $outputPath) {
    throw "Output directory already exists; refusing to overwrite: $outputPath"
}

$inspectionScript = Join-Path $PSScriptRoot 'inspect_grok_install.ps1'
$inspectionArgs = @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', $inspectionScript)
if ($BinaryPath) {
    $inspectionArgs += @('-BinaryPath', $BinaryPath)
}
$inspectionJson = (& powershell @inspectionArgs 2>&1 | Out-String).Trim()
$inspectionExit = $LASTEXITCODE
if ($inspectionExit -ne 0) {
    throw "Grok binary verification failed (exit $inspectionExit): $inspectionJson"
}
$inspection = $inspectionJson | ConvertFrom-Json
if (-not $inspection.valid) {
    throw 'Grok binary verification report was not valid.'
}

$gitStatus = (& git -C $repoRoot status --porcelain=v1 --untracked-files=all 2>$null | Out-String).TrimEnd()
$gitStatusExit = $LASTEXITCODE
if ($gitStatusExit -ne 0) {
    throw "Unable to read Git status for $repoRoot"
}
$previousErrorPreference = $ErrorActionPreference
$ErrorActionPreference = 'Continue'
try {
    $gitHead = (& git -C $repoRoot rev-parse --verify HEAD 2>$null | Out-String).Trim()
    $gitHeadExit = $LASTEXITCODE
} finally {
    $ErrorActionPreference = $previousErrorPreference
}
$gitUnborn = $gitHeadExit -ne 0
if ($gitUnborn) {
    $gitHead = $null
}
$gitStatusLineCount = 0
if ($gitStatus.Length -gt 0) {
    $gitStatusLineCount = @($gitStatus -split "`r?`n").Count
}

$runId = 'DRYRUN-' + ([guid]::NewGuid().ToString('N'))
$sessionId = [guid]::NewGuid().ToString()
$grokHome = Join-Path $outputPath 'grok-home'
$planPath = Join-Path $outputPath 'plan.json'
$stdoutPath = Join-Path $outputPath 'stdout.streaming.jsonl'
$stderrPath = Join-Path $outputPath 'stderr.log'
$debugPath = Join-Path $outputPath 'grok-debug.log'
$journalPath = Join-Path $outputPath 'journal.jsonl'
$tracePath = Join-Path $outputPath 'trace.json'
$exportPath = Join-Path $outputPath 'session.md'

$argv = @(
    $inspection.binary_path,
    '--cwd', $repoRoot,
    '--model', $ModelAlias,
    '--reasoning-effort', $ReasoningEffort,
    '--max-turns', $MaxTurns.ToString([System.Globalization.CultureInfo]::InvariantCulture),
    '--output-format', 'streaming-json',
    '--session-id', $sessionId,
    '--no-memory',
    '--no-subagents',
    '--disable-web-search',
    '--permission-mode', 'plan',
    '--sandbox', $SandboxProfile,
    '--debug-file', $debugPath
)
if ($ToolAllowlist.Count -gt 0) {
    $argv += @('--tools', ($ToolAllowlist -join ','))
}
$argv += @('--prompt-file', $prompt.path)

$credentialPresent = Test-Path -LiteralPath 'Env:LIF_DEEPSEEK_API_KEY'
$plan = [ordered]@{
    schema_version = '0.1.0'
    plan_kind = 'dry-run-no-execution'
    created_at = (Get-Date).ToUniversalTime().ToString('o')
    run_id = $runId
    session_id = $sessionId
    binary = [ordered]@{
        path = $inspection.binary_path
        version_output = $inspection.observed.version_output
        bytes = [long]$inspection.observed.bytes
        sha256 = $inspection.observed.sha256
        authenticode_status = $inspection.observed.authenticode_status
        verified = $true
    }
    workspace = [ordered]@{
        root = $repoRoot
        git_head = $gitHead
        git_unborn = $gitUnborn
        git_status_sha256 = Get-TextSha256 -Text $gitStatus
        git_status_line_count = $gitStatusLineCount
    }
    inputs = [ordered]@{
        prompt_file = $prompt
        config_file = $config
    }
    model = [ordered]@{
        alias = $ModelAlias
        reasoning_effort = $ReasoningEffort
    }
    controls = [ordered]@{
        max_turns = $MaxTurns
        sandbox_profile = $SandboxProfile
        permission_mode = 'plan'
        network_policy = $NetworkPolicy
        tool_allowlist = @($ToolAllowlist)
        memory = $false
        subagents = $false
        web_search = $false
    }
    environment = [ordered]@{
        grok_home_planned = $grokHome
        credential_variable = 'LIF_DEEPSEEK_API_KEY'
        credential_present = $credentialPresent
        secret_values_recorded = $false
    }
    planned_argv = @($argv)
    outputs = [ordered]@{
        plan = $planPath
        stdout = $stdoutPath
        stderr = $stderrPath
        debug = $debugPath
        journal = $journalPath
        trace = $tracePath
        export = $exportPath
    }
    safety = [ordered]@{
        model_invoked = $false
        network_attempted = $false
        tools_executed = $false
        credentials_read = $false
        files_mutated_outside_output = $false
    }
    limitations = @(
        'This artifact is a dry-run plan, not a run manifest, event journal, trace, or evidence of execution.',
        'Grok flag interaction, custom-model discovery, sandbox enforcement, and provider request shape are not verified by this step.',
        'The network policy is recorded intent only until a fake loopback transport and process-level network control are attached.',
        'An empty tool allowlist is recorded but does not prove that the upstream default tool set is disabled.'
    )
}

New-Item -ItemType Directory -Path $outputPath -ErrorAction Stop | Out-Null
$temporaryPath = Join-Path $outputPath ('.plan.' + [guid]::NewGuid().ToString('N') + '.tmp')
$json = $plan | ConvertTo-Json -Depth 12
$utf8NoBom = New-Object System.Text.UTF8Encoding($false)
try {
    [System.IO.File]::WriteAllText($temporaryPath, $json + [Environment]::NewLine, $utf8NoBom)
    Move-Item -LiteralPath $temporaryPath -Destination $planPath -ErrorAction Stop
} finally {
    if (Test-Path -LiteralPath $temporaryPath) {
        Remove-Item -LiteralPath $temporaryPath -Force
    }
}

$plan | ConvertTo-Json -Depth 12
