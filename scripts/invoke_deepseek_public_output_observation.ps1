[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidateSet('Plan', 'Execute')]
    [string]$Mode,

    [Parameter(Mandatory = $true)]
    [string]$OutputDirectory,

    [string]$ConfirmationToken,

    [ValidateRange(10, 120)]
    [int]$TimeoutSeconds = 45
)

$ErrorActionPreference = 'Stop'
$credentialTarget = 'FEP-Agent/DeepSeek'
$modelName = 'deepseek-v4-pro'
$endpoint = 'https://api.deepseek.com/chat/completions'
$marker = 'LIF_DEEPSEEK_PUBLIC_OUTPUT_OBSERVATION_OK'
$repoRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
$outputRoot = [System.IO.Path]::GetFullPath($OutputDirectory)
$planPath = Join-Path $outputRoot 'plan.json'
$resultPath = Join-Path $outputRoot 'api-observation-result.json'
$failurePath = Join-Path $outputRoot 'failure.json'
$pipelineRoot = Join-Path $outputRoot 'projection'

function Write-Utf8Atomic {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][AllowEmptyString()][string]$Content
    )
    $temporary = $Path + '.' + [guid]::NewGuid().ToString('N') + '.tmp'
    $encoding = New-Object System.Text.UTF8Encoding($false)
    try {
        [System.IO.Directory]::CreateDirectory([System.IO.Path]::GetDirectoryName($Path)) | Out-Null
        [System.IO.File]::WriteAllText($temporary, $Content, $encoding)
        Move-Item -LiteralPath $temporary -Destination $Path -ErrorAction Stop
    } finally {
        if (Test-Path -LiteralPath $temporary) {
            Remove-Item -LiteralPath $temporary -Force
        }
    }
}

function Get-Sha256Text {
    param([Parameter(Mandatory = $true)][AllowEmptyString()][string]$Text)
    $bytes = [System.Text.Encoding]::UTF8.GetBytes($Text)
    try {
        $sha = [System.Security.Cryptography.SHA256]::Create()
        try {
            return ([System.BitConverter]::ToString($sha.ComputeHash($bytes))).Replace('-', '').ToLowerInvariant()
        } finally {
            $sha.Dispose()
        }
    } finally {
        if ($bytes.Length -gt 0) {
            [Array]::Clear($bytes, 0, $bytes.Length)
        }
    }
}

function Assert-NoCommonSecretPattern {
    param(
        [Parameter(Mandatory = $true)][string]$Root,
        [AllowEmptyString()][string]$PendingText
    )
    $files = @()
    if (Test-Path -LiteralPath $Root) {
        $files = @(Get-ChildItem -LiteralPath $Root -Recurse -File -Force)
    }
    $pendingIncluded = $PSBoundParameters.ContainsKey('PendingText')
    if ($files.Count + [int]$pendingIncluded -gt 96) {
        throw 'Artifact leak scan exceeded the 96-file bound.'
    }
    $totalBytes = [long]0
    foreach ($file in $files) {
        if ($file.Attributes -band [System.IO.FileAttributes]::ReparsePoint) {
            throw 'Artifact leak scan rejects reparse points.'
        }
        $totalBytes += [long]$file.Length
        if ($totalBytes -gt 8MB) {
            throw 'Artifact leak scan exceeded the 8 MiB bound.'
        }
        $text = Get-Content -LiteralPath $file.FullName -Raw -Encoding UTF8
        if ($null -eq $text) {
            $text = ''
        }
        if ($text -match '(?i)(?<![A-Za-z0-9_-])sk-[A-Za-z0-9_-]{16,}') {
            throw 'Artifact leak scan detected forbidden pattern: deepseek_key_shape.'
        }
        $bearerMatches = [regex]::Matches(
            $text,
            '(?i)(?<![A-Za-z0-9_-])bearer[ \t]+(?<token><[A-Za-z0-9_-]+>|your_[A-Za-z0-9_-]+|\$\{[A-Za-z_][A-Za-z0-9_]*\}|\$[A-Za-z_][A-Za-z0-9_]*|[A-Za-z0-9._~+/=-]{8,})'
        )
        foreach ($bearerMatch in $bearerMatches) {
            $candidate = $bearerMatch.Groups['token'].Value
            if ($candidate -match '(?i)^(?:<[A-Za-z0-9_-]+>|your_[A-Za-z0-9_-]+|\$\{[A-Za-z_][A-Za-z0-9_]*\}|\$[A-Za-z_][A-Za-z0-9_]*)$') {
                continue
            }
            throw 'Artifact leak scan detected forbidden pattern: bearer_credential.'
        }
    }
    if ($pendingIncluded) {
        $totalBytes += [System.Text.Encoding]::UTF8.GetByteCount($PendingText)
        if ($totalBytes -gt 8MB) {
            throw 'Artifact leak scan exceeded the 8 MiB bound.'
        }
        if ($PendingText -match '(?i)(?<![A-Za-z0-9_-])sk-[A-Za-z0-9_-]{16,}') {
            throw 'Artifact leak scan detected forbidden pattern: deepseek_key_shape.'
        }
        $pendingBearerMatches = [regex]::Matches(
            $PendingText,
            '(?i)(?<![A-Za-z0-9_-])bearer[ \t]+(?<token><[A-Za-z0-9_-]+>|your_[A-Za-z0-9_-]+|\$\{[A-Za-z_][A-Za-z0-9_]*\}|\$[A-Za-z_][A-Za-z0-9_]*|[A-Za-z0-9._~+/=-]{8,})'
        )
        foreach ($pendingBearerMatch in $pendingBearerMatches) {
            $candidate = $pendingBearerMatch.Groups['token'].Value
            if ($candidate -match '(?i)^(?:<[A-Za-z0-9_-]+>|your_[A-Za-z0-9_-]+|\$\{[A-Za-z_][A-Za-z0-9_]*\}|\$[A-Za-z_][A-Za-z0-9_]*)$') {
                continue
            }
            throw 'Artifact leak scan detected forbidden pattern: bearer_credential.'
        }
    }
    return [ordered]@{
        policy = 'deepseek-api-observation-artifact-common-secret-patterns-v0.1'
        complete = $true
        scanned_file_count = $files.Count + [int]$pendingIncluded
        pending_document_scanned = $pendingIncluded
        actual_credential_read_for_scan = $false
        hit_count = 0
    }
}

function Get-ConfirmationSummary {
    return [ordered]@{
        schema_version = '0.1.0-draft'
        action = 'deepseek-api-public-output-observation'
        billable_external_request = $true
        provider = 'deepseek'
        endpoint = $endpoint
        model = $modelName
        prompt_sha256 = Get-Sha256Text -Text ("Reply with exactly $marker")
        expected_marker_sha256 = Get-Sha256Text -Text $marker
        max_turns = 1
        max_completion_tokens = 128
        tool_count = 0
        retry_budget = 0
        stream = $false
        web_enabled = $false
        subagents_enabled = $false
        raw_request_recorded = $false
        raw_response_recorded = $false
        credential_source = 'windows-credential-manager-current-user'
        credential_target = $credentialTarget
        credential_value_recorded = $false
    }
}

if ($Mode -eq 'Plan') {
    if (Test-Path -LiteralPath $outputRoot) {
        throw "Output directory already exists; refusing to overwrite: $outputRoot"
    }
    $summary = Get-ConfirmationSummary
    $summaryJson = $summary | ConvertTo-Json -Depth 20 -Compress
    $summarySha256 = Get-Sha256Text -Text $summaryJson
    $plan = [ordered]@{
        schema_version = '0.1.0-draft'
        plan_kind = 'deepseek-api-public-output-observation'
        created_at = [DateTime]::UtcNow.ToString('o')
        confirmation_summary = $summary
        confirmation_summary_sha256 = $summarySha256
        confirmation_token_hint = 'ALLOW-DEEPSEEK-' + $summarySha256.Substring(0, 12).ToUpperInvariant()
        execution = [ordered]@{
            credential_read = $false
            network_attempted = $false
            billable_request_made = $false
            pipeline_projection_started = $false
        }
        limitations = @(
            'This plan is offline and does not prove credential, provider, or billing readiness.',
            'Execution is exactly one fixed prompt request and projects only public assistant output.'
        )
    }
    Write-Utf8Atomic -Path $planPath -Content (($plan | ConvertTo-Json -Depth 30) + [Environment]::NewLine)
    $plan | ConvertTo-Json -Depth 30
    exit 0
}

if ($env:OS -ne 'Windows_NT') {
    throw 'DeepSeek API observation requires Windows Credential Manager.'
}
if (-not (Test-Path -LiteralPath $planPath -PathType Leaf)) {
    throw 'Execute requires an existing plan.json.'
}
if ((Test-Path -LiteralPath $resultPath) -or (Test-Path -LiteralPath $failurePath) -or (Test-Path -LiteralPath $pipelineRoot)) {
    throw 'Refusing to overwrite an existing terminal artifact.'
}

if (-not ('LifDeepSeekApiCredential' -as [type])) {
    Add-Type -TypeDefinition @'
using System;
using System.ComponentModel;
using System.Runtime.InteropServices;

public static class LifDeepSeekApiCredential
{
    private const uint CRED_TYPE_GENERIC = 1;
    private const uint WER_FAULT_REPORTING_FLAG_NOHEAP = 1;

    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    private struct CREDENTIAL {
        public uint Flags; public uint Type; public string TargetName; public string Comment;
        public System.Runtime.InteropServices.ComTypes.FILETIME LastWritten;
        public uint CredentialBlobSize; public IntPtr CredentialBlob; public uint Persist;
        public uint AttributeCount; public IntPtr Attributes; public string TargetAlias; public string UserName;
    }

    [DllImport("Advapi32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern bool CredRead(string target, uint type, uint flags, out IntPtr credential);
    [DllImport("Advapi32.dll")]
    private static extern void CredFree(IntPtr credential);
    [DllImport("Kernel32.dll")]
    private static extern int WerSetFlags(uint flags);
    [DllImport("Kernel32.dll")]
    private static extern int WerGetFlags(IntPtr process, out uint flags);
    [DllImport("Kernel32.dll")]
    private static extern IntPtr GetCurrentProcess();

    public static void ConfigureNoHeapWer() {
        int status = WerSetFlags(WER_FAULT_REPORTING_FLAG_NOHEAP);
        uint flags;
        if (status != 0 || WerGetFlags(GetCurrentProcess(), out flags) != 0 ||
            (flags & WER_FAULT_REPORTING_FLAG_NOHEAP) == 0)
            throw new InvalidOperationException("WER NOHEAP verification failed.");
    }

    public static string Read(string target) {
        IntPtr pointer;
        if (!CredRead(target, CRED_TYPE_GENERIC, 0, out pointer))
            throw new Win32Exception(Marshal.GetLastWin32Error(), "Pinned Windows credential is unavailable.");
        CREDENTIAL credential = new CREDENTIAL();
        try {
            credential = (CREDENTIAL)Marshal.PtrToStructure(pointer, typeof(CREDENTIAL));
            if (credential.CredentialBlobSize == 0 || credential.CredentialBlobSize % 2 != 0)
                throw new InvalidOperationException("Pinned credential blob is not valid UTF-16LE.");
            string secret = Marshal.PtrToStringUni(credential.CredentialBlob, (int)credential.CredentialBlobSize / 2).TrimEnd('\0');
            if (secret.Length < 8 || secret.Length > 512)
                throw new InvalidOperationException("Pinned credential length is outside the accepted bound.");
            foreach (char value in secret)
                if (value < 0x21 || value > 0x7e)
                    throw new InvalidOperationException("Pinned credential must be printable ASCII without whitespace.");
            return secret;
        } finally {
            if (credential.CredentialBlob != IntPtr.Zero)
                for (int index = 0; index < credential.CredentialBlobSize; index++)
                    Marshal.WriteByte(credential.CredentialBlob, index, 0);
            CredFree(pointer);
        }
    }
}
'@
}

$failureStage = 'preflight'
$startedAt = [DateTime]::UtcNow
try {
    $plan = Get-Content -LiteralPath $planPath -Raw -Encoding UTF8 | ConvertFrom-Json
    $summary = Get-ConfirmationSummary
    $summaryJson = $summary | ConvertTo-Json -Depth 20 -Compress
    $summarySha256 = Get-Sha256Text -Text $summaryJson
    if (
        $plan.plan_kind -ne 'deepseek-api-public-output-observation' -or
        $plan.confirmation_summary_sha256 -ne $summarySha256
    ) {
        throw 'Plan no longer matches the fixed launcher summary.'
    }
    $expectedToken = 'ALLOW-DEEPSEEK-' + $summarySha256.Substring(0, 12).ToUpperInvariant()
    if ([string]::IsNullOrWhiteSpace($ConfirmationToken)) {
        $ConfirmationToken = Read-Host "Type $expectedToken to allow exactly one billable DeepSeek request"
    }
    if ($ConfirmationToken -cne $expectedToken) {
        throw 'DeepSeek API observation denied: confirmation token mismatch.'
    }
    $ConfirmationToken = '<cleared>'

    $failureStage = 'process_security'
    [LifDeepSeekApiCredential]::ConfigureNoHeapWer()
    $failureStage = 'credential_read'
    $apiKey = [LifDeepSeekApiCredential]::Read($credentialTarget)
    $failureStage = 'request_render'
    $bodyObject = [ordered]@{
        model = $modelName
        messages = @(
            [ordered]@{
                role = 'user'
                content = "Reply with exactly $marker"
            }
        )
        max_tokens = 128
        temperature = 0
        stream = $false
    }
    $body = $bodyObject | ConvertTo-Json -Depth 10 -Compress
    $headers = @{
        'Authorization' = "Bearer $apiKey"
        'Content-Type' = 'application/json'
    }
    $failureStage = 'network_request'
    $response = Invoke-WebRequest -Uri $endpoint -Method Post -Headers $headers -Body $body -TimeoutSec $TimeoutSeconds -UseBasicParsing
    $headers = $null
    $apiKey = $null
    $failureStage = 'response_parse'
    $document = $response.Content | ConvertFrom-Json
    $message = $document.choices[0].message
    $content = [string]$message.content
    $normalizedContent = $content.Trim()
    $failureStage = 'response_content_validate'
    if ([string]::IsNullOrWhiteSpace($normalizedContent)) {
        throw 'DeepSeek API observation returned empty public content.'
    }
    $markerMatched = ($normalizedContent -eq $marker)
    $result = [ordered]@{
        schema_version = '0.1.0-draft'
        result_kind = 'deepseek_api_observation'
        valid = $true
        started_at = $startedAt.ToString('o')
        completed_at = [DateTime]::UtcNow.ToString('o')
        request_count = 1
        retry_count = 0
        provider = 'deepseek'
        endpoint = $endpoint
        model = $modelName
        http_status_code = [int]$response.StatusCode
        marker_matched = $markerMatched
        public_assistant_text = $normalizedContent
        response_content_sha256 = Get-Sha256Text -Text $normalizedContent
        credential_source = 'windows-credential-manager-current-user'
        credential_target = $credentialTarget
        credential_value_recorded = $false
        raw_response_recorded = $false
        usage = [ordered]@{
            prompt_tokens = [int]$document.usage.prompt_tokens
            completion_tokens = [int]$document.usage.completion_tokens
            total_tokens = [int]$document.usage.total_tokens
        }
        controls = [ordered]@{
            max_completion_tokens = 128
            temperature = 0
            tool_count = 0
            stream = $false
            retry_budget = 0
            web_enabled = $false
            subagents_enabled = $false
            raw_request_recorded = $false
        }
        limitations = @(
            'This is one fixed prompt API observation, not a production runner adapter.',
            'Raw provider response and credential value are not recorded.',
            'Only public assistant content is projected into the downstream guard pipeline.'
        )
    }
    $reasoningContent = $null
    if ($message.PSObject.Properties.Name -contains 'reasoning_content') {
        $reasoningContent = [string]$message.reasoning_content
    }
    if (-not [string]::IsNullOrEmpty($reasoningContent)) {
        $result.private_reasoning_content_sha256 = Get-Sha256Text -Text $reasoningContent
    }
    $document = $null
    $response = $null
    $message = $null
    $normalizedContent = $null
    $reasoningContent = $null
    $body = $null

    $failureStage = 'artifact_leak_scan_prewrite'
    $resultJson = ($result | ConvertTo-Json -Depth 30) + [Environment]::NewLine
    Assert-NoCommonSecretPattern -Root $outputRoot -PendingText $resultJson | Out-Null
    $failureStage = 'result_write'
    Write-Utf8Atomic -Path $resultPath -Content $resultJson
    $failureStage = 'pipeline_projection'
    $projectionJson = (& python (Join-Path $repoRoot 'scripts/build_deepseek_public_output_observation.py') `
        --api-result $resultPath `
        --output-root $pipelineRoot 2>&1 | Out-String).Trim()
    if ($LASTEXITCODE -ne 0) {
        throw 'DeepSeek public-output projection failed.'
    }
    $failureStage = 'artifact_leak_scan_postwrite'
    $leakScan = Assert-NoCommonSecretPattern -Root $outputRoot
    $projection = $projectionJson | ConvertFrom-Json
    $final = [ordered]@{
        valid = $true
        result_path = $resultPath
        projection_path = $pipelineRoot
        public_output_count = [int]$projection.decisions.public_output_count
        stagnation_decision = [string]$projection.decisions.stagnation_decision
        terminal_event_type = [string]$projection.decisions.terminal_event_type
        artifact_leak_scan = $leakScan
    }
    $final | ConvertTo-Json -Depth 20
    exit 0
} catch {
    $failure = [ordered]@{
        schema_version = '0.1.0-draft'
        result_kind = 'deepseek_api_observation_failure'
        valid = $false
        failed_at = [DateTime]::UtcNow.ToString('o')
        stage = $failureStage
        error_type = $_.Exception.GetType().Name
        raw_exception_recorded = $false
        provider_receipt = 'not_recorded'
        billing_status = if ($failureStage -eq 'network_request' -or $failureStage -eq 'response_parse' -or $failureStage -eq 'response_content_validate' -or $failureStage -eq 'artifact_leak_scan_prewrite' -or $failureStage -eq 'result_write' -or $failureStage -eq 'pipeline_projection' -or $failureStage -eq 'artifact_leak_scan_postwrite') { 'possibly_attempted' } else { 'not_attempted' }
        credential_value_recorded = $false
        raw_response_recorded = $false
    }
    $failureJson = ($failure | ConvertTo-Json -Depth 20) + [Environment]::NewLine
    try {
        Assert-NoCommonSecretPattern -Root $outputRoot -PendingText $failureJson | Out-Null
        Write-Utf8Atomic -Path $failurePath -Content $failureJson
    } catch {
    }
    $failure | ConvertTo-Json -Depth 20
    exit 1
}
