[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidateSet('Plan', 'Execute')]
    [string]$Mode,

    [Parameter(Mandatory = $true)]
    [string]$OutputDirectory,

    [string]$ConfirmationToken,

    [string]$BinaryPath,

    [ValidateRange(20, 240)]
    [int]$TimeoutSeconds = 120
)

$ErrorActionPreference = 'Stop'
$credentialTarget = 'orz-deepseek/agent'
$credentialEnvironmentName = 'LIF_DEEPSEEK_API_KEY'
$modelAlias = 'lif-deepseek-v4-pro'
$modelName = 'deepseek-v4-pro'
$baseUrl = 'https://api.deepseek.com'
$marker = 'LIF_GROK_REAL_DEEPSEEK_OK'
$repoRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
$outputRoot = [System.IO.Path]::GetFullPath($OutputDirectory)
$planPath = Join-Path $outputRoot 'plan.json'
$resultPath = Join-Path $outputRoot 'result.json'
$failurePath = Join-Path $outputRoot 'failure.json'

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
        } finally {
            $sha.Dispose()
        }
    } finally {
        [Array]::Clear($bytes, 0, $bytes.Length)
    }
}

function ConvertTo-NativeArgument {
    param([Parameter(Mandatory = $true)][AllowEmptyString()][string]$Value)
    if ($Value.Contains('"')) {
        throw 'Native argument may not contain a quote character.'
    }
    return '"' + $Value + '"'
}

function Join-NativeArguments {
    param(
        [Parameter(Mandatory = $true)]
        [AllowEmptyCollection()]
        [AllowEmptyString()]
        [string[]]$Values
    )
    return (($Values | ForEach-Object { ConvertTo-NativeArgument -Value $_ }) -join ' ')
}

function Get-BinaryInspection {
    $arguments = @(
        '-NoProfile',
        '-ExecutionPolicy', 'Bypass',
        '-File', (Join-Path $PSScriptRoot 'inspect_grok_install.ps1')
    )
    if ($BinaryPath) {
        $arguments += @('-BinaryPath', $BinaryPath)
    }
    $json = (& powershell @arguments 2>&1 | Out-String).Trim()
    if ($LASTEXITCODE -ne 0) {
        throw 'Grok binary inspection failed.'
    }
    $inspection = $json | ConvertFrom-Json
    if (-not $inspection.valid) {
        throw 'Grok binary inspection returned valid=false.'
    }
    return $inspection
}

function Get-ConfirmationSummary {
    param([Parameter(Mandatory = $true)]$Inspection)
    return [ordered]@{
        schema_version = '0.1.0'
        action = 'grok-real-deepseek-conformance'
        billable_external_request = $true
        binary_sha256 = [string]$Inspection.observed.sha256
        binary_version = [string]$Inspection.observed.version_output
        provider = 'deepseek'
        endpoint = 'https://api.deepseek.com/chat/completions'
        model = $modelName
        model_alias = $modelAlias
        prompt_sha256 = Get-Sha256Text -Text ("Reply with exactly $marker")
        expected_marker_sha256 = Get-Sha256Text -Text $marker
        max_turns = 1
        max_completion_tokens = 64
        tool_count = 0
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
}

function New-RestrictedWorkspaceTrustReceipt {
    param(
        [Parameter(Mandatory = $true)][string]$WorkspacePath,
        [Parameter(Mandatory = $true)][string]$ReceiptPath
    )
    $script = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot 'new_grok_workspace_trust_receipt.ps1')).Path
    $json = (& powershell -NoProfile -ExecutionPolicy Bypass -File $script `
        -WorkspacePath $WorkspacePath -OutputPath $ReceiptPath -Decision restricted 2>&1 | Out-String).Trim()
    if ($LASTEXITCODE -ne 0) {
        throw 'Workspace trust receipt generation failed.'
    }
    $receipt = $json | ConvertFrom-Json
    if (
        $receipt.valid -ne $true -or
        $receipt.discovery.complete -ne $true -or
        $receipt.discovery.candidate_count -ne 0 -or
        $receipt.decision.mode -ne 'restricted' -or
        $receipt.decision.launch_permitted -ne $true
    ) {
        throw 'Restricted workspace trust receipt did not permit launch.'
    }
    return $receipt
}

function Assert-NoCommonSecretPattern {
    param(
        [Parameter(Mandatory = $true)][string]$Root,
        [AllowEmptyString()][string]$PendingText
    )
    $files = @(Get-ChildItem -LiteralPath $Root -Recurse -File -Force)
    $pendingIncluded = $PSBoundParameters.ContainsKey('PendingText')
    if ($files.Count + [int]$pendingIncluded -gt 64) {
        throw 'Artifact leak scan exceeded the 64-file bound.'
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
        policy = 'grok-real-artifact-common-secret-patterns-v0.1'
        complete = $true
        scanned_file_count = $files.Count + [int]$pendingIncluded
        pending_document_scanned = $pendingIncluded
        actual_credential_read_for_scan = $false
        hit_count = 0
    }
}

if ($Mode -eq 'Plan') {
    if (Test-Path -LiteralPath $outputRoot) {
        throw "Output directory already exists; refusing to overwrite: $outputRoot"
    }
    $inspection = Get-BinaryInspection
    $summary = Get-ConfirmationSummary -Inspection $inspection
    $summaryJson = $summary | ConvertTo-Json -Depth 20 -Compress
    $summarySha256 = Get-Sha256Text -Text $summaryJson
    $plan = [ordered]@{
        schema_version = '0.1.0'
        plan_kind = 'grok-real-deepseek-conformance'
        created_at = [DateTime]::UtcNow.ToString('o')
        confirmation_summary = $summary
        confirmation_summary_sha256 = $summarySha256
        confirmation_token_hint = 'ALLOW-GROK-' + $summarySha256.Substring(0, 12).ToUpperInvariant()
        execution = [ordered]@{
            credential_read = $false
            grok_started = $false
            network_attempted = $false
            billable_request_made = $false
        }
        limitations = @(
            'This plan is offline and does not prove credential, provider, Grok request-shape, or billing readiness.',
            'Execution is a one-request development conformance and is not a general agent session.'
        )
    }
    New-Item -ItemType Directory -Path $outputRoot | Out-Null
    Write-Utf8Atomic -Path $planPath -Content (($plan | ConvertTo-Json -Depth 30) + [Environment]::NewLine)
    $plan | ConvertTo-Json -Depth 30
    exit 0
}

if ($env:OS -ne 'Windows_NT') {
    throw 'Real Grok DeepSeek conformance requires Windows.'
}
if (-not (Test-Path -LiteralPath $planPath -PathType Leaf)) {
    throw 'Execute requires an existing plan.json.'
}
if ((Test-Path -LiteralPath $resultPath) -or (Test-Path -LiteralPath $failurePath)) {
    throw 'Refusing to overwrite an existing terminal artifact.'
}
$plan = Get-Content -LiteralPath $planPath -Raw -Encoding UTF8 | ConvertFrom-Json
$inspection = Get-BinaryInspection
$summary = Get-ConfirmationSummary -Inspection $inspection
$summaryJson = $summary | ConvertTo-Json -Depth 20 -Compress
$summarySha256 = Get-Sha256Text -Text $summaryJson
if (
    $plan.plan_kind -ne 'grok-real-deepseek-conformance' -or
    $plan.confirmation_summary_sha256 -ne $summarySha256
) {
    throw 'Plan no longer matches the fixed launcher summary.'
}
$expectedToken = 'ALLOW-GROK-' + $summarySha256.Substring(0, 12).ToUpperInvariant()
if ([string]::IsNullOrWhiteSpace($ConfirmationToken)) {
    $ConfirmationToken = Read-Host "Type $expectedToken to allow exactly one billable Grok/DeepSeek request"
}
if ($ConfirmationToken -cne $expectedToken) {
    throw 'Real Grok DeepSeek request denied: confirmation token mismatch.'
}
$ConfirmationToken = '<cleared>'

if (-not ('LifGrokCredentialProcess' -as [type])) {
    Add-Type -TypeDefinition @'
using System;
using System.Collections.Generic;
using System.ComponentModel;
using System.Diagnostics;
using System.Runtime.InteropServices;

public static class LifGrokCredentialProcess
{
    private const uint CRED_TYPE_GENERIC = 1;
    private const uint WER_FAULT_REPORTING_FLAG_NOHEAP = 1;
    private const uint JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE = 0x00002000;
    private const int JobObjectExtendedLimitInformation = 9;

    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    private struct CREDENTIAL {
        public uint Flags; public uint Type; public string TargetName; public string Comment;
        public System.Runtime.InteropServices.ComTypes.FILETIME LastWritten;
        public uint CredentialBlobSize; public IntPtr CredentialBlob; public uint Persist;
        public uint AttributeCount; public IntPtr Attributes; public string TargetAlias; public string UserName;
    }
    [StructLayout(LayoutKind.Sequential)]
    private struct IO_COUNTERS {
        public ulong ReadOperationCount, WriteOperationCount, OtherOperationCount;
        public ulong ReadTransferCount, WriteTransferCount, OtherTransferCount;
    }
    [StructLayout(LayoutKind.Sequential)]
    private struct JOBOBJECT_BASIC_LIMIT_INFORMATION {
        public long PerProcessUserTimeLimit, PerJobUserTimeLimit; public uint LimitFlags;
        public UIntPtr MinimumWorkingSetSize, MaximumWorkingSetSize; public uint ActiveProcessLimit;
        public UIntPtr Affinity; public uint PriorityClass, SchedulingClass;
    }
    [StructLayout(LayoutKind.Sequential)]
    private struct JOBOBJECT_EXTENDED_LIMIT_INFORMATION {
        public JOBOBJECT_BASIC_LIMIT_INFORMATION BasicLimitInformation;
        public IO_COUNTERS IoInfo;
        public UIntPtr ProcessMemoryLimit, JobMemoryLimit, PeakProcessMemoryUsed, PeakJobMemoryUsed;
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
    [DllImport("Kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern IntPtr CreateJobObject(IntPtr attributes, string name);
    [DllImport("Kernel32.dll", SetLastError = true)]
    private static extern bool SetInformationJobObject(IntPtr job, int infoClass, ref JOBOBJECT_EXTENDED_LIMIT_INFORMATION info, uint length);
    [DllImport("Kernel32.dll", SetLastError = true)]
    private static extern bool AssignProcessToJobObject(IntPtr job, IntPtr process);
    [DllImport("Kernel32.dll", SetLastError = true)]
    private static extern bool CloseHandle(IntPtr handle);

    public static void ConfigureNoHeapWer() {
        int status = WerSetFlags(WER_FAULT_REPORTING_FLAG_NOHEAP);
        uint flags;
        if (status != 0 || WerGetFlags(GetCurrentProcess(), out flags) != 0 ||
            (flags & WER_FAULT_REPORTING_FLAG_NOHEAP) == 0)
            throw new InvalidOperationException("WER NOHEAP verification failed.");
    }

    public static IntPtr CreateKillOnCloseJob() {
        IntPtr job = CreateJobObject(IntPtr.Zero, null);
        if (job == IntPtr.Zero) throw new Win32Exception(Marshal.GetLastWin32Error());
        var info = new JOBOBJECT_EXTENDED_LIMIT_INFORMATION();
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        if (!SetInformationJobObject(job, JobObjectExtendedLimitInformation, ref info,
            (uint)Marshal.SizeOf(typeof(JOBOBJECT_EXTENDED_LIMIT_INFORMATION)))) {
            int error = Marshal.GetLastWin32Error(); CloseHandle(job); throw new Win32Exception(error);
        }
        return job;
    }

    public static void Assign(IntPtr job, Process process) {
        if (!AssignProcessToJobObject(job, process.Handle))
            throw new Win32Exception(Marshal.GetLastWin32Error());
    }
    public static void CloseJob(IntPtr job) {
        if (job != IntPtr.Zero && !CloseHandle(job))
            throw new Win32Exception(Marshal.GetLastWin32Error());
    }

    public static Process Start(
        string target, string environmentName, string fileName, string arguments,
        string workingDirectory, Dictionary<string,string> environment) {
        IntPtr pointer;
        if (!CredRead(target, CRED_TYPE_GENERIC, 0, out pointer))
            throw new Win32Exception(Marshal.GetLastWin32Error(), "Pinned Windows credential is unavailable.");
        string secret = null;
        CREDENTIAL credential = new CREDENTIAL();
        try {
            credential = (CREDENTIAL)Marshal.PtrToStructure(pointer, typeof(CREDENTIAL));
            if (credential.CredentialBlobSize == 0 || credential.CredentialBlobSize % 2 != 0)
                throw new InvalidOperationException("Pinned credential blob is not valid UTF-16LE.");
            secret = Marshal.PtrToStringUni(credential.CredentialBlob, (int)credential.CredentialBlobSize / 2).TrimEnd('\0');
            if (secret.Length < 8 || secret.Length > 512)
                throw new InvalidOperationException("Pinned credential length is outside the accepted bound.");
            foreach (char value in secret)
                if (value < 0x21 || value > 0x7e)
                    throw new InvalidOperationException("Pinned credential must be printable ASCII without whitespace.");

            var info = new ProcessStartInfo();
            info.FileName = fileName; info.Arguments = arguments; info.WorkingDirectory = workingDirectory;
            info.UseShellExecute = false; info.CreateNoWindow = true;
            info.RedirectStandardOutput = true; info.RedirectStandardError = true;
            info.EnvironmentVariables.Clear();
            foreach (var item in environment) info.EnvironmentVariables[item.Key] = item.Value;
            info.EnvironmentVariables[environmentName] = secret;
            var process = new Process(); process.StartInfo = info;
            if (!process.Start()) throw new InvalidOperationException("Grok process did not start.");
            info.EnvironmentVariables.Remove(environmentName);
            return process;
        } finally {
            if (credential.CredentialBlob != IntPtr.Zero)
                for (int index = 0; index < credential.CredentialBlobSize; index++)
                    Marshal.WriteByte(credential.CredentialBlob, index, 0);
            secret = null;
            CredFree(pointer);
        }
    }
}
'@
}

$workspace = Join-Path $outputRoot 'workspace'
$profileRoot = Join-Path $outputRoot 'profile'
$configRoot = Join-Path $profileRoot '.grok'
$tempRoot = Join-Path $outputRoot 'temp'
$promptPath = Join-Path $workspace 'prompt.txt'
$configPath = Join-Path $configRoot 'config.toml'
$outputProjectionPath = Join-Path $outputRoot 'grok.output.projection.json'
$preflightPath = Join-Path $outputRoot 'workspace-trust.preflight.json'
$launchPath = Join-Path $outputRoot 'workspace-trust.launch.json'
foreach ($path in @($workspace, $configRoot, $tempRoot)) {
    New-Item -ItemType Directory -Path $path -ErrorAction Stop | Out-Null
}
Write-Utf8Atomic -Path $promptPath -Content ("Reply with exactly $marker" + [Environment]::NewLine)
$config = @"
[cli]
use_leader = false

[model.$modelAlias]
model = "$modelName"
base_url = "$baseUrl"
name = "LIF DeepSeek V4 Pro"
env_key = "$credentialEnvironmentName"
api_backend = "chat_completions"
max_completion_tokens = 64
context_window = 128000
"@
Write-Utf8Atomic -Path $configPath -Content ($config.TrimStart() + [Environment]::NewLine)

$preflight = New-RestrictedWorkspaceTrustReceipt -WorkspacePath $workspace -ReceiptPath $preflightPath
$launch = New-RestrictedWorkspaceTrustReceipt -WorkspacePath $workspace -ReceiptPath $launchPath
if (
    $preflight.discovery.aggregate_sha256 -ne $launch.discovery.aggregate_sha256 -or
    $preflight.discovery.scan_policy_sha256 -ne $launch.discovery.scan_policy_sha256
) {
    throw 'Workspace control surface changed before launch.'
}

$environment = New-Object 'System.Collections.Generic.Dictionary[string,string]'
foreach ($entry in @{
    'SystemRoot' = $env:SystemRoot; 'WINDIR' = $env:WINDIR; 'COMSPEC' = $env:COMSPEC;
    'PATH' = $env:PATH; 'PATHEXT' = $env:PATHEXT; 'OS' = $env:OS;
    'PROCESSOR_ARCHITECTURE' = $env:PROCESSOR_ARCHITECTURE;
    'NUMBER_OF_PROCESSORS' = $env:NUMBER_OF_PROCESSORS;
    'HOME' = $profileRoot; 'USERPROFILE' = $profileRoot; 'APPDATA' = $profileRoot;
    'LOCALAPPDATA' = $profileRoot; 'TEMP' = $tempRoot; 'TMP' = $tempRoot;
    'GROK_SANDBOX' = 'read-only'; 'GROK_MEMORY' = '0'; 'GROK_SUBAGENTS' = '0';
    'GROK_WEB_FETCH' = '0'
}.GetEnumerator()) {
    $environment.Add([string]$entry.Key, [string]$entry.Value)
}
$sessionId = [guid]::NewGuid().ToString()
$arguments = @(
    '--cwd', $workspace,
    '--model', $modelAlias,
    '--reasoning-effort', 'high',
    '--max-turns', '1',
    '--output-format', 'streaming-json',
    '--session-id', $sessionId,
    '--no-memory',
    '--no-subagents',
    '--disable-web-search',
    '--permission-mode', 'plan',
    '--sandbox', 'read-only',
    '--tools', '',
    '--verbatim',
    '--prompt-file', $promptPath
)

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
        $credentialTarget,
        $credentialEnvironmentName,
        [string]$inspection.binary_path,
        $nativeArguments,
        $outputRoot,
        $environment
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
        throw 'Grok real conformance timed out.'
    }
    $stdout = $stdoutTask.Result
    $stderr = $stderrTask.Result
    $failureStage = 'output_projection_write'
    $outputProjection = [ordered]@{
        stdout_bytes = [System.Text.Encoding]::UTF8.GetByteCount($stdout)
        stdout_sha256 = Get-Sha256Text -Text $stdout
        stderr_bytes = [System.Text.Encoding]::UTF8.GetByteCount($stderr)
        stderr_sha256 = Get-Sha256Text -Text $stderr
        raw_stdout_recorded = $false
        raw_stderr_recorded = $false
    }
    Write-Utf8Atomic -Path $outputProjectionPath -Content (($outputProjection | ConvertTo-Json -Depth 5) + [Environment]::NewLine)
    if ($process.ExitCode -ne 0) {
        throw 'Grok real conformance exited nonzero.'
    }
    $failureStage = 'streaming_output_validate'
    $events = @(
        $stdout -split "`r?`n" |
            Where-Object { -not [string]::IsNullOrWhiteSpace($_) } |
            ForEach-Object { $_ | ConvertFrom-Json }
    )
    $textEvents = @($events | Where-Object { $_.type -eq 'text' })
    $endEvents = @($events | Where-Object { $_.type -eq 'end' })
    if ($textEvents.Count -ne 1 -or [string]$textEvents[0].data -cne $marker -or $endEvents.Count -ne 1) {
        throw 'Grok streaming output did not contain the exact terminal marker sequence.'
    }
    $end = $endEvents[0]
    $failureStage = 'artifact_leak_scan'
    $leakScan = Assert-NoCommonSecretPattern -Root $outputRoot
    $failureStage = 'result_write'
    $result = [ordered]@{
        schema_version = '0.1.0'
        result_kind = 'grok-real-deepseek-conformance'
        valid = $true
        started_at = $startedAt.ToString('o')
        completed_at = [DateTime]::UtcNow.ToString('o')
        confirmation_summary_sha256 = $summarySha256
        request_count = 1
        retry_count = 0
        provider = 'deepseek'
        endpoint = 'https://api.deepseek.com/chat/completions'
        model = $modelName
        marker_matched = $true
        response_content_recorded = $false
        credential_value_recorded = $false
        process = [ordered]@{
            exit_code = [int]$process.ExitCode
            duration_ms = [math]::Round($stopwatch.Elapsed.TotalMilliseconds, 3)
            job_object_created = $true
            job_object_assigned = $true
            kill_on_close = $true
            assignment_race_known = $true
            stdout_bytes = [int]$outputProjection.stdout_bytes
            stdout_sha256 = [string]$outputProjection.stdout_sha256
            stderr_bytes = [int]$outputProjection.stderr_bytes
            stderr_sha256 = [string]$outputProjection.stderr_sha256
            raw_stdout_recorded = $false
            raw_stderr_recorded = $false
        }
        usage = [ordered]@{
            input_tokens = [int]$end.usage.input_tokens
            output_tokens = [int]$end.usage.output_tokens
            reasoning_tokens = [int]$end.usage.reasoning_tokens
            total_tokens = [int]$end.usage.total_tokens
            model_calls = [int]$end.modelUsage.$modelName.modelCalls
        }
        controls = [ordered]@{
            binary_locked = $true
            workspace_restricted = $true
            workspace_receipts_stable = $true
            isolated_profile = $true
            clean_environment = $true
            proxy_environment_used = $false
            debug_enabled = $false
            tools_enabled = $false
            memory_enabled = $false
            subagents_enabled = $false
            web_enabled = $false
            sandbox = 'read-only'
            wer_noheap_verified_for_launcher = $true
        }
        artifact_leak_scan = $leakScan
        limitations = @(
            'This is one fixed Grok-to-DeepSeek development conformance, not a general agent session or reliability evaluation.',
            'The DeepSeek key necessarily exists in the Grok child environment; launcher WER NOHEAP does not disable child-process crash dumps.',
            'The application-level leak scan does not cover pagefile, hibernation, administrator, debugger, malware, kernel buffers, or provider systems.',
            'Job Object assignment occurs immediately after process start and retains the known pre-assignment race.'
        )
    }
    $pendingResult = ($result | ConvertTo-Json -Depth 30) + [Environment]::NewLine
    $failureStage = 'artifact_leak_scan'
    $result.artifact_leak_scan = Assert-NoCommonSecretPattern -Root $outputRoot -PendingText $pendingResult
    $finalResult = ($result | ConvertTo-Json -Depth 30) + [Environment]::NewLine
    $verifiedResultScan = Assert-NoCommonSecretPattern -Root $outputRoot -PendingText $finalResult
    if ($verifiedResultScan.scanned_file_count -ne $result.artifact_leak_scan.scanned_file_count) {
        throw 'Pending result leak-scan file count changed.'
    }
    $failureStage = 'result_write'
    Write-Utf8Atomic -Path $resultPath -Content $finalResult
    $result | ConvertTo-Json -Depth 30
    exit 0
} catch {
    if ($failureStage -eq 'artifact_leak_scan') {
        throw 'Grok real DeepSeek conformance artifact leak scan failed; no terminal artifact was written.'
    }
    $failure = [ordered]@{
        schema_version = '0.1.0'
        result_kind = 'grok-real-deepseek-conformance-failure'
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
        $failure.artifact_leak_scan = Assert-NoCommonSecretPattern -Root $outputRoot -PendingText $pendingFailure
        $finalFailure = ($failure | ConvertTo-Json -Depth 10) + [Environment]::NewLine
        $verifiedFailureScan = Assert-NoCommonSecretPattern -Root $outputRoot -PendingText $finalFailure
        if ($verifiedFailureScan.scanned_file_count -ne $failure.artifact_leak_scan.scanned_file_count) {
            throw 'Pending failure leak-scan file count changed.'
        }
        Write-Utf8Atomic -Path $failurePath -Content $finalFailure
    }
    throw 'Grok real DeepSeek conformance failed; inspect the sanitized failure artifact.'
} finally {
    if ($job -ne [IntPtr]::Zero -and -not $jobClosed) {
        try {
            [LifGrokCredentialProcess]::CloseJob($job)
            $jobClosed = $true
        } catch {
        }
    }
    if ($process -and -not $process.HasExited) {
        Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
    }
    $environment.Clear()
}
