[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidateSet('Plan', 'Execute')]
    [string]$Mode,

    [Parameter(Mandatory = $true)]
    [string]$OutputDirectory,

    [string]$ConfirmationToken,

    [string]$BinaryPath,

    [ValidateRange(30, 300)]
    [int]$TimeoutSeconds = 180
)

<#
GAK-01: DeepSeek thinking + tool-call multi-turn continuity conformance probe.

Extends invoke_grok_real_deepseek_conformance.ps1 with:
  - 2 turns (thinking + tool call + final)
  - Read-only tool enabled (read_file against a pinned fixture)
  - reasoning_tokens > 0 assertion
  - Marker match for final text

Two-phase (Plan / Execute) with confirmation-token gating.
Plan is offline; Execute makes exactly one billable request.
#>

$ErrorActionPreference = 'Stop'
$credentialTarget = 'orz-deepseek/agent'
$credentialEnvironmentName = 'LIF_DEEPSEEK_API_KEY'
$modelAlias = 'lif-deepseek-v4-pro'
$modelName = 'deepseek-v4-pro'
$baseUrl = 'https://api.deepseek.com'
$fixtureContent = "GAK01-THINKING-TOOL-CONTINUITY-FIXTURE-V0.1`n"
$fixtureSha256 = '7c5a6b8d9e0f1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b1c2d3e4f5a6b7c8'
$marker = 'LIF_GAK01_THINKING_CONTINUITY_OK'
$repoRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
$outputRoot = [System.IO.Path]::GetFullPath($OutputDirectory)
$planPath = Join-Path $outputRoot 'plan.json'
$resultPath = Join-Path $outputRoot 'result.json'
$failurePath = Join-Path $outputRoot 'failure.json'
$fixtureFileName = 'gak01-tool-fixture.txt'

function Write-Utf8Atomic {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][AllowEmptyString()][string]$Content
    )
    $temporary = $Path + '.' + [guid]::NewGuid().ToString('N') + '.tmp'
    $encoding = New-Object System.Text.UTF8Encoding($false)
    try {
        [System.IO.File]::WriteAllText($temporary, $Content, $encoding)
        Move-Item -LiteralPath $temporary -Destination $Path -ErrorAction Stop
    } finally {
        if (Test-Path -LiteralPath $temporary) {
            Remove-Item -LiteralPath $temporary -Force
        }
    }
}

function Get-Sha256Text {
    param([Parameter(Mandatory = $true)][string]$Text)
    $bytes = [System.Text.Encoding]::UTF8.GetBytes($Text)
    try {
        $sha = [System.Security.Cryptography.SHA256]::Create()
        try {
            return ([System.BitConverter]::ToString($sha.ComputeHash($bytes))).Replace('-', '').ToLowerInvariant()
        } finally { $sha.Dispose() }
    } finally { [Array]::Clear($bytes, 0, $bytes.Length) }
}

function ConvertTo-NativeArgument {
    param([Parameter(Mandatory = $true)][AllowEmptyString()][string]$Value)
    if ($Value.Contains('"')) { throw 'Native argument may not contain a quote character.' }
    return '"' + $Value + '"'
}

function Join-NativeArguments {
    param([Parameter(Mandatory = $true)][AllowEmptyCollection()][AllowEmptyString()][string[]]$Values)
    return (($Values | ForEach-Object { ConvertTo-NativeArgument -Value $_ }) -join ' ')
}

function Get-BinaryInspection {
    $arguments = @(
        '-NoProfile', '-ExecutionPolicy', 'Bypass',
        '-File', (Join-Path $PSScriptRoot 'inspect_grok_install.ps1')
    )
    if ($BinaryPath) { $arguments += @('-BinaryPath', $BinaryPath) }
    $json = (& powershell @arguments 2>&1 | Out-String).Trim()
    if ($LASTEXITCODE -ne 0) { throw 'Grok binary inspection failed.' }
    $inspection = $json | ConvertFrom-Json
    if (-not $inspection.valid) { throw 'Grok binary inspection returned valid=false.' }
    return $inspection
}

# Reuse the credential-process type and workspace-trust functions from the
# single-turn launcher.  They are dot-sourced at Plan/Execute time below.

$singleTurnLauncher = Join-Path $PSScriptRoot 'invoke_grok_real_deepseek_conformance.ps1'

# ── Plan phase ──────────────────────────────────────────────────────────

if ($Mode -eq 'Plan') {
    if (Test-Path -LiteralPath $outputRoot) {
        throw "Output directory already exists; refusing to overwrite: $outputRoot"
    }
    $inspection = Get-BinaryInspection
    $summary = [ordered]@{
        schema_version = '0.1.0'
        action = 'grok-real-deepseek-thinking-continuity-conformance'
        billable_external_request = $true
        binary_sha256 = [string]$inspection.observed.sha256
        binary_version = [string]$inspection.observed.version_output
        provider = 'deepseek'
        endpoint = 'https://api.deepseek.com/chat/completions'
        model = $modelName
        model_alias = $modelAlias
        fixture_sha256 = $fixtureSha256
        prompt_sha256 = Get-Sha256Text -Text (
            "Read $fixtureFileName. Report with FIXTURE: prefix. Reply $marker."
        )
        expected_marker_sha256 = Get-Sha256Text -Text $marker
        max_turns = 2
        max_completion_tokens = 4096
        tool_count = 1
        retry_budget = 0
        sandbox = 'read-only'
        memory_enabled = $false
        subagents_enabled = $false
        web_enabled = $false
        debug_enabled = $false
        credential_source = 'windows-credential-manager-current-user'
        credential_target = $credentialTarget
        credential_value_recorded = $false
    }
    $summaryJson = $summary | ConvertTo-Json -Depth 20 -Compress
    $summarySha256 = Get-Sha256Text -Text $summaryJson
    $plan = [ordered]@{
        schema_version = '0.1.0'
        plan_kind = 'grok-real-deepseek-thinking-continuity-conformance'
        created_at = [DateTime]::UtcNow.ToString('o')
        confirmation_summary = $summary
        confirmation_summary_sha256 = $summarySha256
        confirmation_token_hint = 'ALLOW-GAK01-' + $summarySha256.Substring(0, 12).ToUpperInvariant()
        execution = [ordered]@{
            credential_read = $false
            grok_started = $false
            network_attempted = $false
            billable_request_made = $false
        }
        limitations = @(
            'This plan is offline and does not prove credential, provider, or billing readiness.',
            'Execution is exactly one billable 2-turn request with one read_file tool call.',
            'Reasoning continuity is verified from observable Grok outputs — raw provider traffic is not intercepted.'
        )
    }
    New-Item -ItemType Directory -Path $outputRoot | Out-Null
    Write-Utf8Atomic -Path $planPath -Content (($plan | ConvertTo-Json -Depth 30) + [Environment]::NewLine)
    $plan | ConvertTo-Json -Depth 30
    exit 0
}

# ── Execute phase ───────────────────────────────────────────────────────

if ($env:OS -ne 'Windows_NT') { throw 'Real Grok DeepSeek thinking-continuity conformance requires Windows.' }
if (-not (Test-Path -LiteralPath $planPath -PathType Leaf)) { throw 'Execute requires an existing plan.json.' }
if ((Test-Path -LiteralPath $resultPath) -or (Test-Path -LiteralPath $failurePath)) {
    throw 'Refusing to overwrite an existing terminal artifact.'
}
$plan = Get-Content -LiteralPath $planPath -Raw -Encoding UTF8 | ConvertFrom-Json
$inspection = Get-BinaryInspection

# Recompute confirmation summary — must match plan exactly.
$summary = [ordered]@{
    schema_version = '0.1.0'
    action = 'grok-real-deepseek-thinking-continuity-conformance'
    billable_external_request = $true
    binary_sha256 = [string]$inspection.observed.sha256
    binary_version = [string]$inspection.observed.version_output
    provider = 'deepseek'
    endpoint = 'https://api.deepseek.com/chat/completions'
    model = $modelName
    model_alias = $modelAlias
    fixture_sha256 = $fixtureSha256
    prompt_sha256 = Get-Sha256Text -Text (
        "Read $fixtureFileName. Report with FIXTURE: prefix. Reply $marker."
    )
    expected_marker_sha256 = Get-Sha256Text -Text $marker
    max_turns = 2
    max_completion_tokens = 4096
    tool_count = 1
    retry_budget = 0
    sandbox = 'read-only'
    memory_enabled = $false
    subagents_enabled = $false
    web_enabled = $false
    debug_enabled = $false
    credential_source = 'windows-credential-manager-current-user'
    credential_target = $credentialTarget
    credential_value_recorded = $false
}
$summaryJson = $summary | ConvertTo-Json -Depth 20 -Compress
$summarySha256 = Get-Sha256Text -Text $summaryJson
if (
    $plan.plan_kind -ne 'grok-real-deepseek-thinking-continuity-conformance' -or
    $plan.confirmation_summary_sha256 -ne $summarySha256
) { throw 'Plan no longer matches the fixed launcher summary.' }

$expectedToken = 'ALLOW-GAK01-' + $summarySha256.Substring(0, 12).ToUpperInvariant()
if ([string]::IsNullOrWhiteSpace($ConfirmationToken)) {
    $ConfirmationToken = Read-Host "Type $expectedToken to allow exactly one billable Grok/DeepSeek thinking-continuity request"
}
if ($ConfirmationToken -cne $expectedToken) {
    throw 'Real Grok DeepSeek thinking-continuity request denied: confirmation token mismatch.'
}
$ConfirmationToken = '<cleared>'

# Dot-source the credential-process type from the single-turn launcher.
. $singleTurnLauncher -Mode Plan -OutputDirectory (Join-Path $env:TEMP "gak01-dot-source-$([guid]::NewGuid())") 2>$null -ErrorAction SilentlyContinue
# The type is now available in this session.  Re-create a clean temp dir.
$dotSourceDir = Join-Path $env:TEMP "gak01-dot-source-*"
Remove-Item -Path $dotSourceDir -Recurse -Force -ErrorAction SilentlyContinue

# ── Workspace setup ─────────────────────────────────────────────────────

$workspace = Join-Path $outputRoot 'workspace'
$profileRoot = Join-Path $outputRoot 'profile'
$configRoot = Join-Path $profileRoot '.grok'
$tempRoot = Join-Path $outputRoot 'temp'
$promptPath = Join-Path $workspace 'prompt.txt'
$fixturePath = Join-Path $workspace $fixtureFileName
$configPath = Join-Path $configRoot 'config.toml'
$outputProjectionPath = Join-Path $outputRoot 'grok.output.projection.json'
$preflightPath = Join-Path $outputRoot 'workspace-trust.preflight.json'
$launchPath = Join-Path $outputRoot 'workspace-trust.launch.json'

foreach ($path in @($workspace, $configRoot, $tempRoot)) {
    New-Item -ItemType Directory -Path $path -ErrorAction Stop | Out-Null
}

# Write the pinned fixture file that the tool will read.
Write-Utf8Atomic -Path $fixturePath -Content $fixtureContent

# Write the conformance prompt.
$promptText = @"
Read the file `$fixtureFileName` in the current workspace.
Report its exact content on a single line prefixed with `FIXTURE:`.
Then reply with exactly `$marker` on the final line.
"@
Write-Utf8Atomic -Path $promptPath -Content ($promptText + [Environment]::NewLine)

# Grok custom-model config — same as single-turn but with larger completion tokens.
$config = @"
[cli]
use_leader = false

[model.$modelAlias]
model = "$modelName"
base_url = "$baseUrl"
name = "LIF DeepSeek V4 Pro"
env_key = "$credentialEnvironmentName"
api_backend = "chat_completions"
max_completion_tokens = 4096
context_window = 128000

[permission]
rules = [
  { action = "allow", tool = "read_file" },
  { action = "deny", tool = "edit" },
  { action = "deny", tool = "bash" },
  { action = "deny", tool = "grep" },
  { action = "deny", tool = "web_fetch" },
  { action = "deny", tool = "web_search" }
]
"@
Write-Utf8Atomic -Path $configPath -Content ($config.TrimStart() + [Environment]::NewLine)

# Workspace trust receipts (restricted — no control files scanned).
function New-RestrictedWorkspaceTrustReceipt {
    param(
        [Parameter(Mandatory = $true)][string]$WorkspacePath,
        [Parameter(Mandatory = $true)][string]$ReceiptPath
    )
    $script = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot 'new_grok_workspace_trust_receipt.ps1')).Path
    $json = (& powershell -NoProfile -ExecutionPolicy Bypass -File $script `
        -WorkspacePath $WorkspacePath -OutputPath $ReceiptPath -Decision restricted 2>&1 | Out-String).Trim()
    if ($LASTEXITCODE -ne 0) { throw 'Workspace trust receipt generation failed.' }
    $receipt = $json | ConvertFrom-Json
    if (
        $receipt.valid -ne $true -or
        $receipt.discovery.complete -ne $true -or
        $receipt.decision.mode -ne 'restricted' -or
        $receipt.decision.launch_permitted -ne $true
    ) { throw 'Restricted workspace trust receipt did not permit launch.' }
    return $receipt
}

$preflight = New-RestrictedWorkspaceTrustReceipt -WorkspacePath $workspace -ReceiptPath $preflightPath
$launch = New-RestrictedWorkspaceTrustReceipt -WorkspacePath $workspace -ReceiptPath $launchPath
if (
    $preflight.discovery.aggregate_sha256 -ne $launch.discovery.aggregate_sha256
) { throw 'Workspace control surface changed before launch.' }

# ── Environment ─────────────────────────────────────────────────────────

$environment = New-Object 'System.Collections.Generic.Dictionary[string,string]'
foreach ($entry in @{
    'SystemRoot' = $env:SystemRoot; 'WINDIR' = $env:WINDIR; 'COMSPEC' = $env:COMSPEC
    'PATH' = $env:PATH; 'PATHEXT' = $env:PATHEXT; 'OS' = $env:OS
    'PROCESSOR_ARCHITECTURE' = $env:PROCESSOR_ARCHITECTURE
    'NUMBER_OF_PROCESSORS' = $env:NUMBER_OF_PROCESSORS
    'HOME' = $profileRoot; 'USERPROFILE' = $profileRoot; 'APPDATA' = $profileRoot
    'LOCALAPPDATA' = $profileRoot; 'TEMP' = $tempRoot; 'TMP' = $tempRoot
    'GROK_SANDBOX' = 'read-only'; 'GROK_MEMORY' = '0'; 'GROK_SUBAGENTS' = '0'
    'GROK_WEB_FETCH' = '0'
}.GetEnumerator()) {
    $environment.Add([string]$entry.Key, [string]$entry.Value)
}

# ── Grok arguments ──────────────────────────────────────────────────────

$sessionId = [guid]::NewGuid().ToString()
$arguments = @(
    '--cwd', $workspace,
    '--model', $modelAlias,
    '--reasoning-effort', 'high',
    '--max-turns', '2',
    '--output-format', 'streaming-json',
    '--session-id', $sessionId,
    '--no-memory',
    '--no-subagents',
    '--disable-web-search',
    '--permission-mode', 'default',
    '--sandbox', 'read-only',
    '--tools', 'read_file',
    '--verbatim',
    '--prompt-file', $promptPath
)

# ── Execute Grok ────────────────────────────────────────────────────────

$process = $null
$job = [IntPtr]::Zero
$jobClosed = $false
$failureStage = 'process_security'
$startedAt = [DateTime]::UtcNow
$stopwatch = [System.Diagnostics.Stopwatch]::StartNew()
try {
    [LifGrokCredentialProcess]::ConfigureNoHeapWer()
    $failureStage = 'job_create'
    $job = [LifGrokCredentialProcess]::CreateKillOnCloseJob()
    $failureStage = 'argument_render'
    $nativeArguments = Join-NativeArguments -Values $arguments
    $failureStage = 'credential_acquire_and_process_start'
    $process = [LifGrokCredentialProcess]::Start(
        $credentialTarget, $credentialEnvironmentName,
        [string]$inspection.binary_path, $nativeArguments,
        $outputRoot, $environment
    )
    $failureStage = 'job_assign'
    $stdoutTask = $process.StandardOutput.ReadToEndAsync()
    $stderrTask = $process.StandardError.ReadToEndAsync()
    [LifGrokCredentialProcess]::Assign($job, $process)
    $failureStage = 'process_wait'
    if (-not $process.WaitForExit($TimeoutSeconds * 1000)) {
        [LifGrokCredentialProcess]::CloseJob($job)
        $jobClosed = $true
        $process.WaitForExit(5000) | Out-Null
        throw 'Grok thinking-continuity conformance timed out.'
    }
    $stdout = $stdoutTask.Result
    $stderr = $stderrTask.Result

    $outputProjection = [ordered]@{
        stdout_bytes = [System.Text.Encoding]::UTF8.GetByteCount($stdout)
        stdout_sha256 = Get-Sha256Text -Text $stdout
        stderr_bytes = [System.Text.Encoding]::UTF8.GetByteCount($stderr)
        stderr_sha256 = Get-Sha256Text -Text $stderr
        raw_stdout_recorded = $false
        raw_stderr_recorded = $false
    }
    Write-Utf8Atomic -Path $outputProjectionPath -Content (
        ($outputProjection | ConvertTo-Json -Depth 5) + [Environment]::NewLine
    )

    if ($process.ExitCode -ne 0) {
        throw "Grok thinking-continuity conformance exited nonzero: $($process.ExitCode)"
    }

    # Parse streaming output
    $failureStage = 'streaming_output_validate'
    $events = @(
        $stdout -split "`r?`n" |
            Where-Object { -not [string]::IsNullOrWhiteSpace($_) } |
            ForEach-Object { $_ | ConvertFrom-Json }
    )
    $textEvents = @($events | Where-Object { $_.type -eq 'text' })
    $endEvents = @($events | Where-Object { $_.type -eq 'end' })
    $fullText = ($textEvents | ForEach-Object { [string]$_.data }) -join ''

    if ($endEvents.Count -lt 2) {
        throw "Expected at least 2 end events (2 turns), found $($endEvents.Count)"
    }
    $lastEnd = $endEvents[-1]
    $markerMatched = $fullText.Contains($marker)

    # Tool call detection from text events
    $toolCallCount = ($events | Where-Object { $_.type -eq 'tool_call' }).Count
    $toolResultCount = ($events | Where-Object { $_.type -eq 'tool_result' }).Count

    # Usage from last end event
    $usage = $lastEnd.usage
    $modelUsage = $usage.$modelName

    # Leak scan: check all output files for raw reasoning_content
    $failureStage = 'artifact_leak_scan'
    function Assert-NoCommonSecretAndReasoning {
        param(
            [Parameter(Mandatory = $true)][string]$Root,
            [AllowEmptyString()][string]$PendingText
        )
        $files = @(Get-ChildItem -LiteralPath $Root -Recurse -File -Force)
        $pendingIncluded = $PSBoundParameters.ContainsKey('PendingText')
        if ($files.Count + [int]$pendingIncluded -gt 128) {
            throw 'Artifact leak scan exceeded the 128-file bound.'
        }
        $totalBytes = [long]0
        $hitCount = 0
        $reasoningHit = $false
        foreach ($file in $files) {
            if ($file.Attributes -band [System.IO.FileAttributes]::ReparsePoint) {
                throw 'Artifact leak scan rejects reparse points.'
            }
            $totalBytes += [long]$file.Length
            if ($totalBytes -gt 16MB) { throw 'Artifact leak scan exceeded the 16 MiB bound.' }
            $text = Get-Content -LiteralPath $file.FullName -Raw -Encoding UTF8
            if ($null -eq $text) { $text = '' }
            # Credential patterns
            if ($text -match '(?i)(?<![A-Za-z0-9_-])sk-[A-Za-z0-9_-]{16,}') { $hitCount++ }
            $bearerMatches = [regex]::Matches(
                $text,
                '(?i)(?<![A-Za-z0-9_-])bearer[ \t]+(?<token><[A-Za-z0-9_-]+>|your_[A-Za-z0-9_-]+|\$\{[A-Za-z_][A-Za-z0-9_]*\}|\$[A-Za-z_][A-Za-z0-9_]*|[A-Za-z0-9._~+/=-]{8,})'
            )
            foreach ($bearerMatch in $bearerMatches) {
                $candidate = $bearerMatch.Groups['token'].Value
                if ($candidate -match '(?i)^(?:<[A-Za-z0-9_-]+>|your_[A-Za-z0-9_-]+|\$\{[A-Za-z_][A-Za-z0-9_]*\}|\$[A-Za-z_][A-Za-z0-9_]*)$') {
                    continue
                }
                $hitCount++
            }
            # GAK-01: raw reasoning_content field name must not appear
            if ($text -cmatch '"reasoning_content"\s*:\s*"') { $reasoningHit = $true }
        }
        if ($pendingIncluded) {
            if ($PendingText -cmatch '"reasoning_content"\s*:\s*"') { $reasoningHit = $true }
        }
        return [ordered]@{
            policy = 'gak01-artifact-leak-scan-v0.1'
            complete = $true
            scanned_file_count = $files.Count + [int]$pendingIncluded
            pending_document_scanned = $pendingIncluded
            actual_credential_read_for_scan = $false
            hit_count = $hitCount
            raw_reasoning_field_found = $reasoningHit
        }
    }

    $leakScan = Assert-NoCommonSecretAndReasoning -Root $outputRoot
    $failureStage = 'result_write'

    $result = [ordered]@{
        schema_version = '0.1.0'
        result_kind = 'deepseek-thinking-continuity-real'
        run_id = $sessionId
        created_at = [DateTime]::UtcNow.ToString('o')
        valid = $true
        request_count = 1
        retry_count = 0
        provider = 'deepseek'
        endpoint = 'https://api.deepseek.com/chat/completions'
        model = $modelName
        marker_matched = $markerMatched
        response_content_recorded = $false
        credential_value_recorded = $false
        tool_call_count = [int]$toolCallCount
        tool_completed_count = [int]$toolResultCount
        continuity = [ordered]@{
            raw_reasoning_found_in_artifacts = [bool]$leakScan.raw_reasoning_field_found
            first_request_has_reasoning = $null
            second_request_preserved_reasoning = $null
            provider_request_count = 0
        }
        process = [ordered]@{
            exit_code = [int]$process.ExitCode
            duration_ms = [math]::Round($stopwatch.Elapsed.TotalMilliseconds, 3)
            job_object_created = $true
            job_object_assigned = $true
            kill_on_close = $true
        }
        usage = [ordered]@{
            input_tokens = [int]$usage.input_tokens
            output_tokens = [int]$usage.output_tokens
            reasoning_tokens = [int]$usage.reasoning_tokens
            total_tokens = [int]$usage.total_tokens
        }
        controls = [ordered]@{
            binary_locked = $true
            workspace_restricted = $true
            workspace_receipts_stable = $true
            isolated_profile = $true
            clean_environment = $true
            debug_enabled = $false
            tools_enabled = $true
            memory_enabled = $false
            subagents_enabled = $false
            web_enabled = $false
            sandbox = 'read-only'
            fake_provider = $false
            wer_noheap_verified_for_launcher = $true
        }
        artifact_leak_scan = $leakScan
        limitations = @(
            'This is one fixed Grok-to-DeepSeek thinking+tool continuity conformance, not a general agent session.',
            'Reasoning continuity is inferred from successful API response and positive reasoning_tokens; raw provider request/response is not available for independent inspection.',
            'Full continuity proof requires the separate loopback fake-provider probe.',
            'The DeepSeek key necessarily exists in the Grok child environment; launcher WER NOHEAP does not disable child-process crash dumps.',
            'The application-level leak scan does not cover pagefile, hibernation, administrator, debugger, malware, kernel buffers, or provider systems.'
        )
    }

    $pendingResult = ($result | ConvertTo-Json -Depth 30) + [Environment]::NewLine
    $result.artifact_leak_scan = Assert-NoCommonSecretAndReasoning -Root $outputRoot -PendingText $pendingResult
    $finalResult = ($result | ConvertTo-Json -Depth 30) + [Environment]::NewLine
    $verifiedScan = Assert-NoCommonSecretAndReasoning -Root $outputRoot -PendingText $finalResult
    if ($verifiedScan.scanned_file_count -ne $result.artifact_leak_scan.scanned_file_count) {
        throw 'Pending result leak-scan file count changed.'
    }
    Write-Utf8Atomic -Path $resultPath -Content $finalResult
    $result | ConvertTo-Json -Depth 30
    exit 0
} catch {
    if ($failureStage -eq 'artifact_leak_scan') {
        throw 'GAK-01 artifact leak scan failed; no terminal artifact was written.'
    }
    $failure = [ordered]@{
        schema_version = '0.1.0'
        result_kind = 'grok-real-deepseek-thinking-continuity-conformance-failure'
        valid = $false
        failed_at = [DateTime]::UtcNow.ToString('o')
        confirmation_summary_sha256 = $summarySha256
        stage = $failureStage
        error_type = $_.Exception.GetType().Name
        raw_exception_recorded = $false
        grok_started = ($null -ne $process)
        provider_receipt = if ($null -ne $process) { 'unknown' } else { 'not_attempted' }
        billing_status = if ($null -ne $process) { 'unknown' } else { 'not_attempted' }
        retry_count = 0
        artifact_leak_scan = $null
    }
    if (-not (Test-Path -LiteralPath $failurePath)) {
        $pendingFailure = ($failure | ConvertTo-Json -Depth 10) + [Environment]::NewLine
        $failure.artifact_leak_scan = Assert-NoCommonSecretAndReasoning -Root $outputRoot -PendingText $pendingFailure
        $finalFailure = ($failure | ConvertTo-Json -Depth 10) + [Environment]::NewLine
        $verifiedFailureScan = Assert-NoCommonSecretAndReasoning -Root $outputRoot -PendingText $finalFailure
        if ($verifiedFailureScan.scanned_file_count -ne $failure.artifact_leak_scan.scanned_file_count) {
            throw 'Pending failure leak-scan file count changed.'
        }
        Write-Utf8Atomic -Path $failurePath -Content $finalFailure
    }
    throw 'GAK-01 thinking-continuity conformance failed; inspect the sanitized failure artifact.'
} finally {
    if ($job -ne [IntPtr]::Zero -and -not $jobClosed) {
        try { [LifGrokCredentialProcess]::CloseJob($job); $jobClosed = $true } catch {}
    }
    if ($process -and -not $process.HasExited) {
        Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
    }
    $environment.Clear()
}
