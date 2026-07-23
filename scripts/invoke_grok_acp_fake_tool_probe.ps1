[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$OutputDirectory,

    [ValidateSet('allow_once', 'cancel_permission')]
    [string]$Scenario = 'allow_once',

    [ValidateRange(10, 120)]
    [int]$TimeoutSeconds = 45,

    [string]$BinaryPath,

    [string]$PythonPath
)

$ErrorActionPreference = 'Stop'
if ($env:OS -ne 'Windows_NT') {
    throw 'The Grok ACP fake-tool probe is Windows-only.'
}
$repoRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
$outputRoot = [System.IO.Path]::GetFullPath($OutputDirectory)
if (Test-Path -LiteralPath $outputRoot) {
    throw "Output directory already exists; refusing to overwrite: $outputRoot"
}
if ($outputRoot.Contains('"')) {
    throw 'Output directory may not contain a quote character.'
}
$identity = [System.Security.Principal.WindowsIdentity]::GetCurrent()
$principal = New-Object System.Security.Principal.WindowsPrincipal($identity)
if (-not $principal.IsInRole([System.Security.Principal.WindowsBuiltInRole]::Administrator)) {
    throw 'Administrator token required: temporary outbound firewall rules are fail-closed.'
}
foreach ($command in @('Get-NetFirewallProfile', 'Get-NetFirewallRule', 'New-NetFirewallRule', 'Remove-NetFirewallRule')) {
    if (-not (Get-Command $command -ErrorAction SilentlyContinue)) {
        throw "Required Windows Firewall command is unavailable: $command"
    }
}

if (-not ('LifAcpToolJobObject' -as [type])) {
    Add-Type -TypeDefinition @'
using System;
using System.ComponentModel;
using System.Runtime.InteropServices;

public static class LifAcpToolJobObject
{
    private const uint JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE = 0x00002000;
    private const int JobObjectExtendedLimitInformation = 9;

    [StructLayout(LayoutKind.Sequential)]
    private struct IO_COUNTERS
    {
        public ulong ReadOperationCount;
        public ulong WriteOperationCount;
        public ulong OtherOperationCount;
        public ulong ReadTransferCount;
        public ulong WriteTransferCount;
        public ulong OtherTransferCount;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct JOBOBJECT_BASIC_LIMIT_INFORMATION
    {
        public long PerProcessUserTimeLimit;
        public long PerJobUserTimeLimit;
        public uint LimitFlags;
        public UIntPtr MinimumWorkingSetSize;
        public UIntPtr MaximumWorkingSetSize;
        public uint ActiveProcessLimit;
        public UIntPtr Affinity;
        public uint PriorityClass;
        public uint SchedulingClass;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct JOBOBJECT_EXTENDED_LIMIT_INFORMATION
    {
        public JOBOBJECT_BASIC_LIMIT_INFORMATION BasicLimitInformation;
        public IO_COUNTERS IoInfo;
        public UIntPtr ProcessMemoryLimit;
        public UIntPtr JobMemoryLimit;
        public UIntPtr PeakProcessMemoryUsed;
        public UIntPtr PeakJobMemoryUsed;
    }

    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern IntPtr CreateJobObject(IntPtr securityAttributes, string name);

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern bool SetInformationJobObject(
        IntPtr job,
        int informationClass,
        ref JOBOBJECT_EXTENDED_LIMIT_INFORMATION information,
        uint informationLength);

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern bool AssignProcessToJobObject(IntPtr job, IntPtr process);

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern bool CloseHandle(IntPtr handle);

    public static IntPtr CreateKillOnClose()
    {
        IntPtr job = CreateJobObject(IntPtr.Zero, null);
        if (job == IntPtr.Zero) throw new Win32Exception(Marshal.GetLastWin32Error());
        var information = new JOBOBJECT_EXTENDED_LIMIT_INFORMATION();
        information.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        uint size = (uint)Marshal.SizeOf(typeof(JOBOBJECT_EXTENDED_LIMIT_INFORMATION));
        if (!SetInformationJobObject(job, JobObjectExtendedLimitInformation, ref information, size))
        {
            int error = Marshal.GetLastWin32Error();
            CloseHandle(job);
            throw new Win32Exception(error);
        }
        return job;
    }

    public static void Assign(IntPtr job, IntPtr process)
    {
        if (!AssignProcessToJobObject(job, process))
            throw new Win32Exception(Marshal.GetLastWin32Error());
    }

    public static void Close(IntPtr job)
    {
        if (job != IntPtr.Zero && !CloseHandle(job))
            throw new Win32Exception(Marshal.GetLastWin32Error());
    }
}
'@
}

function ConvertTo-NativeArgument {
    param([Parameter(Mandatory = $true)][AllowEmptyString()][string]$Value)
    if ($Value.Contains('"')) {
        throw 'Native argument may not contain a quote character.'
    }
    return '"' + $Value + '"'
}

function Join-NativeArguments {
    param([Parameter(Mandatory = $true)][string[]]$Values)
    return (($Values | ForEach-Object { ConvertTo-NativeArgument -Value $_ }) -join ' ')
}

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

function Get-ArtifactRecord {
    param([Parameter(Mandatory = $true)][string]$Path)
    $item = Get-Item -LiteralPath $Path -ErrorAction Stop
    return [ordered]@{
        path = $item.FullName
        bytes = [long]$item.Length
        sha256 = (Get-FileHash -LiteralPath $item.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
    }
}

function Get-WorkspaceDigest {
    param([Parameter(Mandatory = $true)][string]$WorkspacePath)
    $records = @(
        Get-ChildItem -LiteralPath $WorkspacePath -Recurse -File |
            Sort-Object FullName |
            ForEach-Object {
                $relative = $_.FullName.Substring($WorkspacePath.Length).TrimStart('\').Replace('\', '/')
                $hash = (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
                "$relative`t$($_.Length)`t$hash"
            }
    )
    $bytes = [System.Text.Encoding]::UTF8.GetBytes(($records -join "`n"))
    $sha = [System.Security.Cryptography.SHA256]::Create()
    try {
        return ([BitConverter]::ToString($sha.ComputeHash($bytes))).Replace('-', '').ToLowerInvariant()
    } finally {
        $sha.Dispose()
    }
}

function New-RestrictedWorkspaceTrustReceipt {
    param(
        [Parameter(Mandatory = $true)][string]$WorkspacePath,
        [Parameter(Mandatory = $true)][string]$ReceiptPath
    )
    $script = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot 'new_grok_workspace_trust_receipt.ps1')).Path
    $json = (& powershell -NoProfile -ExecutionPolicy Bypass -File $script `
        -WorkspacePath $WorkspacePath `
        -OutputPath $ReceiptPath `
        -Decision restricted 2>&1 | Out-String).Trim()
    if ($LASTEXITCODE -ne 0) {
        throw "Workspace trust preflight failed: $json"
    }
    $receipt = $json | ConvertFrom-Json
    $canonical = (Resolve-Path -LiteralPath $WorkspacePath).Path
    if (
        $receipt.schema_version -ne '0.1.0' -or
        $receipt.valid -ne $true -or
        $receipt.discovery.complete -ne $true -or
        $receipt.discovery.candidate_count -ne 0 -or
        $receipt.decision.mode -ne 'restricted' -or
        $receipt.decision.launch_permitted -ne $true -or
        -not $receipt.workspace.canonical_path.Equals($canonical, [System.StringComparison]::OrdinalIgnoreCase)
    ) {
        throw 'Workspace trust receipt did not satisfy the restricted zero-candidate policy.'
    }
    return $receipt
}

function Start-RedirectedProcess {
    param(
        [Parameter(Mandatory = $true)][string]$FilePath,
        [Parameter(Mandatory = $true)][string[]]$Arguments,
        [Parameter(Mandatory = $true)][string]$WorkingDirectory,
        [hashtable]$Environment,
        [switch]$CleanEnvironment,
        [switch]$AssignToJob
    )
    $info = New-Object System.Diagnostics.ProcessStartInfo
    $info.FileName = $FilePath
    $info.Arguments = Join-NativeArguments -Values $Arguments
    $info.WorkingDirectory = $WorkingDirectory
    $info.UseShellExecute = $false
    $info.CreateNoWindow = $true
    $info.RedirectStandardOutput = $true
    $info.RedirectStandardError = $true
    if ($CleanEnvironment) {
        $info.EnvironmentVariables.Clear()
    }
    if ($Environment) {
        foreach ($name in $Environment.Keys) {
            if ($null -ne $Environment[$name]) {
                $info.EnvironmentVariables[$name] = [string]$Environment[$name]
            }
        }
    }
    $process = New-Object System.Diagnostics.Process
    $process.StartInfo = $info
    if (-not $process.Start()) {
        throw "Failed to start process: $FilePath"
    }
    $job = [IntPtr]::Zero
    $created = $false
    $assigned = $false
    if ($AssignToJob) {
        try {
            $job = [LifAcpToolJobObject]::CreateKillOnClose()
            $created = $true
            [LifAcpToolJobObject]::Assign($job, $process.Handle)
            $assigned = $true
        } catch {
            if ($job -ne [IntPtr]::Zero) {
                [LifAcpToolJobObject]::Close($job)
            }
            if (-not $process.HasExited) {
                $process.Kill()
                $process.WaitForExit()
            }
            throw
        }
    }
    return [ordered]@{
        process = $process
        stdout_task = $process.StandardOutput.ReadToEndAsync()
        stderr_task = $process.StandardError.ReadToEndAsync()
        job_handle = $job
        job_created = $created
        job_assigned = $assigned
    }
}

if (-not $PythonPath) {
    $PythonPath = (Get-Command python -ErrorAction Stop).Source
}
$PythonPath = (Resolve-Path -LiteralPath $PythonPath).Path
$providerScript = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot 'fake_deepseek_provider.py')).Path
$clientScript = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot 'run_grok_acp_fake_tool_client.py')).Path
$verifyScript = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot 'verify_grok_acp_fake_tool_probe.py')).Path
$promptSource = (Resolve-Path -LiteralPath (Join-Path $repoRoot 'integration\grok\examples\tool-continuity-prompt.txt')).Path
$toolSource = (Resolve-Path -LiteralPath (Join-Path $repoRoot 'integration\grok\examples\tool-continuity-fixture.txt')).Path

$probeId = 'ACPTOOL-' + [guid]::NewGuid().ToString('N')
$startedAt = [DateTime]::UtcNow.ToString('o')
$stopwatch = [System.Diagnostics.Stopwatch]::StartNew()
$profileRoot = Join-Path $outputRoot 'profile'
$configDirectory = Join-Path $profileRoot '.grok'
$configPath = Join-Path $configDirectory 'config.toml'
$workspace = Join-Path $outputRoot 'workspace'
$tempRoot = Join-Path $outputRoot 'temp'
$providerDirectory = Join-Path $outputRoot 'provider'
$clientDirectory = Join-Path $outputRoot 'client'
$readyPath = Join-Path $outputRoot 'client-ready.json'
$startSignalPath = Join-Path $outputRoot 'client-start.signal'
$promptPath = Join-Path $workspace 'prompt.txt'
$toolPath = Join-Path $workspace 'tool-fixture.txt'
$markerPath = Join-Path $workspace '.lif-disposable-workspace.json'
$resultPath = Join-Path $outputRoot 'result.json'
$verificationPath = Join-Path $outputRoot 'verification.json'
$preflightPath = Join-Path $outputRoot 'workspace-trust.preflight.json'
$launchPath = Join-Path $outputRoot 'workspace-trust.launch.json'
$postrunPath = Join-Path $outputRoot 'workspace-trust.postrun.json'
$failurePath = Join-Path $outputRoot 'failure.json'

New-Item -ItemType Directory -Path $outputRoot | Out-Null
New-Item -ItemType Directory -Path $configDirectory | Out-Null
New-Item -ItemType Directory -Path $workspace | Out-Null
New-Item -ItemType Directory -Path $tempRoot | Out-Null
Copy-Item -LiteralPath $promptSource -Destination $promptPath
Copy-Item -LiteralPath $toolSource -Destination $toolPath
$marker = [ordered]@{
    schema_version = '0.1.0'
    marker_kind = 'lif-disposable-workspace'
    fixture_id = 'FIXTURE-' + [guid]::NewGuid().ToString('N')
    disposable = $true
}
Write-Utf8Atomic -Path $markerPath -Content (($marker | ConvertTo-Json -Depth 10) + [Environment]::NewLine)
$workspaceDigestBefore = Get-WorkspaceDigest -WorkspacePath $workspace

$providerHandle = $null
$clientHandle = $null
$inspection = $null
$firewallRules = New-Object System.Collections.Generic.List[string]
$firewallRemoved = $false
$jobClosed = $false
$remainingRuleCount = -1

try {
    $preflight = New-RestrictedWorkspaceTrustReceipt -WorkspacePath $workspace -ReceiptPath $preflightPath
    $inspectionArgs = @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', (Join-Path $PSScriptRoot 'inspect_grok_install.ps1'))
    if ($BinaryPath) {
        $inspectionArgs += @('-BinaryPath', $BinaryPath)
    }
    $inspectionJson = (& powershell @inspectionArgs 2>&1 | Out-String).Trim()
    if ($LASTEXITCODE -ne 0) {
        throw "Grok binary verification failed: $inspectionJson"
    }
    $inspection = $inspectionJson | ConvertFrom-Json
    $upstreamLock = Get-Content -LiteralPath (Join-Path $repoRoot 'upstream\grok-build.lock.json') -Raw -Encoding UTF8 | ConvertFrom-Json

    $providerScenario = if ($Scenario -eq 'allow_once') { 'tool-continuity' } else { 'tool-cancel' }
    $providerHandle = Start-RedirectedProcess `
        -FilePath $PythonPath `
        -Arguments @(
            $providerScript,
            '--output-directory', $providerDirectory,
            '--timeout-seconds', ([string]($TimeoutSeconds + 15)),
            '--scenario', $providerScenario,
            '--tool-fixture-path', $toolPath
        ) `
        -WorkingDirectory $outputRoot

    $providerReadyPath = Join-Path $providerDirectory 'ready.json'
    $providerDeadline = [DateTime]::UtcNow.AddSeconds(10)
    while (-not (Test-Path -LiteralPath $providerReadyPath)) {
        if ($providerHandle.process.HasExited) {
            throw "Fake provider exited before readiness: $($providerHandle.stderr_task.Result)"
        }
        if ([DateTime]::UtcNow -gt $providerDeadline) {
            throw 'Timed out waiting for fake-provider readiness.'
        }
        Start-Sleep -Milliseconds 100
    }
    $providerReady = Get-Content -LiteralPath $providerReadyPath -Raw -Encoding UTF8 | ConvertFrom-Json
    if (-not $providerReady.ready -or $providerReady.host -ne '127.0.0.1' -or $providerReady.external_bind) {
        throw 'Fake provider did not prove a loopback-only bind.'
    }

    $config = @"
[cli]
auto_update = false
use_leader = false

[features]
telemetry = false
feedback = false
lsp_tools = false
codebase_indexing = false
remote_fetch = false

[session]
load_envrc = false

[models]
default = "lif-fake-deepseek"
default_reasoning_effort = "high"

[model.lif-fake-deepseek]
model = "deepseek-v4-pro"
base_url = "http://127.0.0.1:$($providerReady.port)"
name = "LIF Fake DeepSeek Loopback"
env_key = "LIF_FAKE_DEEPSEEK_KEY"
api_backend = "chat_completions"
max_completion_tokens = 256
context_window = 4096

[permission]
rules = [
  { action = "ask", tool = "read" },
  { action = "deny", tool = "edit" },
  { action = "deny", tool = "bash" },
  { action = "deny", tool = "grep" },
  { action = "deny", tool = "mcp" },
  { action = "deny", tool = "webfetch" },
  { action = "deny", tool = "websearch" },
]
"@
    Write-Utf8Atomic -Path $configPath -Content ($config.TrimStart() + [Environment]::NewLine)

    $profiles = @(Get-NetFirewallProfile -ErrorAction Stop)
    if ($profiles.Count -eq 0 -or @($profiles | Where-Object { -not $_.Enabled }).Count -gt 0) {
        throw 'All Windows Firewall profiles must be enabled before the ACP fake-tool probe.'
    }
    $rulePrefix = 'LIFGrokACPTool-' + $probeId.Substring(8)
    $firewallSpecs = @(
        @{ Name = "$rulePrefix-v4-nonloop"; Remote = @(
            '0.0.0.0/2', '64.0.0.0/3', '96.0.0.0/4', '112.0.0.0/5',
            '120.0.0.0/6', '124.0.0.0/7', '126.0.0.0/8', '128.0.0.0/1'
        ) },
        @{ Name = "$rulePrefix-v6-low"; Remote = '::/1' },
        @{ Name = "$rulePrefix-v6-high"; Remote = '8000::/1' }
    )
    foreach ($spec in $firewallSpecs) {
        New-NetFirewallRule `
            -Name $spec.Name `
            -DisplayName $spec.Name `
            -Direction Outbound `
            -Action Block `
            -Enabled True `
            -Profile Any `
            -Program $inspection.binary_path `
            -Protocol Any `
            -RemoteAddress $spec.Remote | Out-Null
        $firewallRules.Add($spec.Name)
    }

    $cleanEnvironment = @{
        'SystemRoot' = $env:SystemRoot
        'WINDIR' = $env:WINDIR
        'COMSPEC' = $env:COMSPEC
        'PATH' = $env:PATH
        'PATHEXT' = $env:PATHEXT
        'OS' = $env:OS
        'PROCESSOR_ARCHITECTURE' = $env:PROCESSOR_ARCHITECTURE
        'NUMBER_OF_PROCESSORS' = $env:NUMBER_OF_PROCESSORS
        'HOME' = $profileRoot
        'USERPROFILE' = $profileRoot
        'APPDATA' = $profileRoot
        'LOCALAPPDATA' = $profileRoot
        'TEMP' = $tempRoot
        'TMP' = $tempRoot
        'LIF_FAKE_DEEPSEEK_KEY' = 'loopback-fixture-not-a-secret'
        'GROK_SANDBOX' = 'read-only'
        'GROK_MEMORY' = '0'
        'GROK_SUBAGENTS' = '0'
        'GROK_WEB_FETCH' = '0'
        'HTTP_PROXY' = 'http://127.0.0.1:1'
        'HTTPS_PROXY' = 'http://127.0.0.1:1'
        'ALL_PROXY' = 'http://127.0.0.1:1'
        'NO_PROXY' = '127.0.0.1,localhost'
    }

    $launch = New-RestrictedWorkspaceTrustReceipt -WorkspacePath $workspace -ReceiptPath $launchPath
    if (
        $launch.discovery.aggregate_sha256 -ne $preflight.discovery.aggregate_sha256 -or
        $launch.discovery.scan_policy_sha256 -ne $preflight.discovery.scan_policy_sha256
    ) {
        throw 'Workspace control surface changed between preflight and launch.'
    }

    $clientHandle = Start-RedirectedProcess `
        -FilePath $PythonPath `
        -Arguments @(
            $clientScript,
            '--binary', $inspection.binary_path,
            '--workspace', $workspace,
            '--profile-root', $profileRoot,
            '--prompt-file', $promptPath,
            '--output-directory', $clientDirectory,
            '--scenario', $Scenario,
            '--timeout-seconds', ([string]$TimeoutSeconds),
            '--ready-path', $readyPath,
            '--start-signal', $startSignalPath
        ) `
        -WorkingDirectory $outputRoot `
        -Environment $cleanEnvironment `
        -CleanEnvironment `
        -AssignToJob

    $clientReadyDeadline = [DateTime]::UtcNow.AddSeconds(10)
    while (-not (Test-Path -LiteralPath $readyPath)) {
        if ($clientHandle.process.HasExited) {
            throw "ACP client exited before readiness: $($clientHandle.stderr_task.Result)"
        }
        if ([DateTime]::UtcNow -gt $clientReadyDeadline) {
            throw 'Timed out waiting for ACP client readiness.'
        }
        Start-Sleep -Milliseconds 50
    }
    Write-Utf8Atomic -Path $startSignalPath -Content "go`n"

    $timedOut = -not $clientHandle.process.WaitForExit($TimeoutSeconds * 1000)
    if ($timedOut) {
        [LifAcpToolJobObject]::Close($clientHandle.job_handle)
        $jobClosed = $true
        if (-not $clientHandle.process.WaitForExit(5000)) {
            Stop-Process -Id $clientHandle.process.Id -Force
            $clientHandle.process.WaitForExit()
        }
        throw 'ACP client timed out.'
    }
    $clientStdout = $clientHandle.stdout_task.Result
    $clientStderr = $clientHandle.stderr_task.Result
    if ($clientHandle.process.ExitCode -ne 0) {
        throw "ACP client failed: $clientStderr"
    }
    $clientResultPath = Join-Path $clientDirectory 'client-result.json'
    $transcriptPath = Join-Path $clientDirectory 'transcript.private.jsonl'
    $clientResult = Get-Content -LiteralPath $clientResultPath -Raw -Encoding UTF8 | ConvertFrom-Json
    if (-not $clientResult.grok_process.inherited_job_at_start) {
        throw 'Grok child process did not inherit the kill-on-close Job Object.'
    }

    if (-not $jobClosed) {
        [LifAcpToolJobObject]::Close($clientHandle.job_handle)
        $jobClosed = $true
    }
    if (-not $providerHandle.process.WaitForExit(($TimeoutSeconds + 15) * 1000)) {
        Stop-Process -Id $providerHandle.process.Id -Force
        $providerHandle.process.WaitForExit()
        throw 'Fake provider timed out.'
    }
    $providerStdout = $providerHandle.stdout_task.Result
    $providerStderr = $providerHandle.stderr_task.Result
    if ($providerHandle.process.ExitCode -ne 0) {
        throw "Fake provider failed: $providerStderr"
    }

    $postrun = New-RestrictedWorkspaceTrustReceipt -WorkspacePath $workspace -ReceiptPath $postrunPath
    if (
        $postrun.discovery.aggregate_sha256 -ne $preflight.discovery.aggregate_sha256 -or
        $postrun.discovery.scan_policy_sha256 -ne $preflight.discovery.scan_policy_sha256
    ) {
        throw 'Workspace control surface changed during the ACP probe.'
    }
} catch {
    $failure = [ordered]@{
        schema_version = '0.1.0'
        failure_kind = 'grok-acp-fake-tool-probe'
        probe_id = $probeId
        scenario = $Scenario
        message = $_.Exception.Message
        started_at = $startedAt
        failed_at = [DateTime]::UtcNow.ToString('o')
        firewall_rule_names = @($firewallRules)
        job_created = if ($clientHandle) { $clientHandle.job_created } else { $false }
        job_assigned = if ($clientHandle) { $clientHandle.job_assigned } else { $false }
    }
    Write-Utf8Atomic -Path $failurePath -Content (($failure | ConvertTo-Json -Depth 20) + [Environment]::NewLine)
    throw
} finally {
    if ($clientHandle -and -not $jobClosed -and $clientHandle.job_handle -ne [IntPtr]::Zero) {
        try {
            [LifAcpToolJobObject]::Close($clientHandle.job_handle)
            $jobClosed = $true
        } catch {
        }
    }
    if ($clientHandle -and -not $clientHandle.process.HasExited) {
        Stop-Process -Id $clientHandle.process.Id -Force -ErrorAction SilentlyContinue
    }
    if ($providerHandle -and -not $providerHandle.process.HasExited) {
        Stop-Process -Id $providerHandle.process.Id -Force -ErrorAction SilentlyContinue
    }
    foreach ($name in $firewallRules) {
        Remove-NetFirewallRule -Name $name -ErrorAction SilentlyContinue
    }
    $remainingRuleCount = @(
        foreach ($name in $firewallRules) {
            Get-NetFirewallRule -Name $name -ErrorAction SilentlyContinue
        }
    ).Count
    $firewallRemoved = $remainingRuleCount -eq 0
}

if (-not $firewallRemoved) {
    throw 'Temporary ACP fake-tool firewall rules were not fully removed.'
}

$providerResultPath = Join-Path $providerDirectory 'provider-result.json'
$providerPrivatePath = Join-Path $providerDirectory 'requests.private.jsonl'
$providerResult = Get-Content -LiteralPath $providerResultPath -Raw -Encoding UTF8 | ConvertFrom-Json
$workspaceDigestAfter = Get-WorkspaceDigest -WorkspacePath $workspace
$workspaceModified = $workspaceDigestBefore -ne $workspaceDigestAfter

$continuity = $providerResult.continuity
$providerProjection = [ordered]@{
    scenario = [string]$providerResult.scenario
    terminal_state = [string]$providerResult.terminal_state
    primary_request_count = [int]$providerResult.primary_request_count
    auxiliary_request_count = [int]$providerResult.auxiliary_request_count
    second_request_observed = if ($Scenario -eq 'allow_once') { [bool]$continuity.second_request_observed } else { $false }
    reasoning_marker_preserved = if ($Scenario -eq 'allow_once') { [bool]$continuity.reasoning_marker_preserved } else { $false }
    tool_call_id_preserved = if ($Scenario -eq 'allow_once') { [bool]$continuity.tool_call_id_preserved } else { $false }
    tool_result_observed = if ($Scenario -eq 'allow_once') { [bool]$continuity.tool_result_observed } else { $false }
    tool_result_marker_observed = if ($Scenario -eq 'allow_once') { [bool]$continuity.tool_result_marker_observed } else { $false }
    real_model_invoked = [bool]$providerResult.response.real_model_invoked
}

$rawCredential = 'loopback-fixture-not-a-secret'
$credentialRecorded = $false
foreach ($path in @($transcriptPath, $providerResultPath, $clientResultPath, (Join-Path $clientDirectory 'grok.stderr.log'))) {
    if ((Get-Content -LiteralPath $path -Raw -Encoding UTF8).Contains($rawCredential)) {
        $credentialRecorded = $true
    }
}

$acp = $clientResult.acp
$sessionEvidence = $clientResult.session_evidence
$commonChecks = [ordered]@{
    binary_matches_lock = (
        $inspection.valid -eq $true -and
        $inspection.observed.sha256 -eq $upstreamLock.binary_release.sha256 -and
        $inspection.observed.bytes -eq $upstreamLock.binary_release.bytes -and
        $inspection.observed.authenticode_status -eq $upstreamLock.binary_release.authenticode_status
    )
    trust_receipts_stable = (
        $preflight.discovery.aggregate_sha256 -eq $launch.discovery.aggregate_sha256 -and
        $launch.discovery.aggregate_sha256 -eq $postrun.discovery.aggregate_sha256 -and
        $preflight.discovery.scan_policy_sha256 -eq $launch.discovery.scan_policy_sha256 -and
        $launch.discovery.scan_policy_sha256 -eq $postrun.discovery.scan_policy_sha256
    )
    provider_loopback_only = ($providerResult.bind.host -eq '127.0.0.1' -and -not $providerResult.bind.external)
    provider_succeeded = ($providerResult.terminal_state -eq 'succeeded')
    acp_request_sequence = (@($acp.request_methods) -join ',') -eq 'initialize,session/new,session/prompt'
    one_tool_call = ($acp.tool_call_count -eq 1 -and -not [string]::IsNullOrWhiteSpace([string]$acp.tool_call_id))
    one_permission_request = ($acp.permission_request_count -eq 1)
    one_prompt_terminal = ($acp.prompt_response_count -eq 1)
    session_files_match = (@($sessionEvidence.tool_call_ids) -contains [string]$acp.tool_call_id)
    no_unexpected_acp = ($acp.unexpected_message_count -eq 0 -and $acp.parse_failure_count -eq 0)
    job_contained = ($clientHandle.job_created -and $clientHandle.job_assigned -and $jobClosed -and $clientResult.grok_process.inherited_job_at_start)
    firewall_cleaned = ($firewallRules.Count -eq 3 -and $firewallRemoved -and $remainingRuleCount -eq 0)
    workspace_unchanged = (-not $workspaceModified)
    credential_not_recorded = (-not $credentialRecorded)
}
if ($Scenario -eq 'allow_once') {
    $scenarioChecks = [ordered]@{
        allow_once_selected = ($acp.permission_outcome -eq 'allow_once' -and -not $acp.cancel_notification_sent)
        tool_has_unique_terminal = ($acp.terminal_tool_update_count -eq 1 -and @($acp.tool_statuses) -contains 'completed')
        prompt_ended_normally = ($acp.prompt_stop_reason -eq 'end_turn')
        provider_continuity_complete = (
            $providerProjection.primary_request_count -eq 2 -and
            $providerProjection.second_request_observed -and
            $providerProjection.reasoning_marker_preserved -and
            $providerProjection.tool_call_id_preserved -and
            $providerProjection.tool_result_observed -and
            $providerProjection.tool_result_marker_observed
        )
        session_tool_completed_once = ($sessionEvidence.completed_tool_event_count -eq 1 -and @($sessionEvidence.permission_decisions) -contains 'allow')
    }
} else {
    $scenarioChecks = [ordered]@{
        pending_permission_cancelled = ($acp.permission_outcome -eq 'cancelled' -and $acp.cancel_notification_sent)
        prompt_cancelled = ($acp.prompt_stop_reason -eq 'cancelled')
        provider_stopped_after_first_request = ($providerProjection.primary_request_count -eq 1 -and -not $providerProjection.second_request_observed)
        no_completed_tool = ($sessionEvidence.completed_tool_event_count -eq 0)
    }
}
$checks = [ordered]@{}
foreach ($entry in $commonChecks.GetEnumerator()) {
    $checks[$entry.Key] = [bool]$entry.Value
}
foreach ($entry in $scenarioChecks.GetEnumerator()) {
    $checks[$entry.Key] = [bool]$entry.Value
}
$valid = @($checks.Values | Where-Object { -not $_ }).Count -eq 0

$result = [ordered]@{
    schema_version = '0.1.0'
    probe_kind = 'grok-acp-fake-tool-continuity'
    probe_id = $probeId
    scenario = $Scenario
    valid = $valid
    started_at = $startedAt
    completed_at = [DateTime]::UtcNow.ToString('o')
    binary = [ordered]@{
        path = [string]$inspection.binary_path
        bytes = [long]$inspection.observed.bytes
        sha256 = [string]$inspection.observed.sha256
        locked_sha256 = [string]$upstreamLock.binary_release.sha256
        version = [string]$upstreamLock.binary_release.version
        authenticode_status = [string]$inspection.observed.authenticode_status
    }
    artifacts = [ordered]@{
        transcript = Get-ArtifactRecord -Path $transcriptPath
        provider_result = Get-ArtifactRecord -Path $providerResultPath
        provider_private_capture = Get-ArtifactRecord -Path $providerPrivatePath
        workspace_trust_preflight = Get-ArtifactRecord -Path $preflightPath
        workspace_trust_launch = Get-ArtifactRecord -Path $launchPath
        workspace_trust_postrun = Get-ArtifactRecord -Path $postrunPath
        session_events = Get-ArtifactRecord -Path ([string]$clientResult.session_events_path)
        session_updates = Get-ArtifactRecord -Path ([string]$clientResult.session_updates_path)
    }
    acp = $acp
    session_evidence = $sessionEvidence
    provider = $providerProjection
    process = [ordered]@{
        exit_code = [int]$clientHandle.process.ExitCode
        timed_out = $false
        duration_ms = [double]$stopwatch.Elapsed.TotalMilliseconds
        job_object_created = [bool]$clientHandle.job_created
        job_object_assigned = [bool]$clientHandle.job_assigned
        job_object_closed = $jobClosed
    }
    controls = [ordered]@{
        loopback_only_provider = ($providerResult.bind.host -eq '127.0.0.1' -and -not $providerResult.bind.external)
        clean_environment = $true
        credential_value_recorded = $credentialRecorded
        outbound_block_created = ($firewallRules.Count -eq 3)
        outbound_rules_removed = $firewallRemoved
        remaining_rule_count = $remainingRuleCount
        workspace_disposable = [bool]$marker.disposable
        workspace_modified = $workspaceModified
        output_overwritten = $false
    }
    checks = $checks
    limitations = @(
        'This probe uses a fixed loopback fake provider and a disposable read-only fixture; it does not call or validate the real DeepSeek service.',
        'ACP and session-store records are cross-checked by identifiers and lifecycle fields, but no cross-source global timestamp order is claimed.',
        'The firewall rules cover the verified Grok executable path; this read-only fixture does not claim descendant-executable network isolation.'
    )
}
Write-Utf8Atomic -Path $resultPath -Content (($result | ConvertTo-Json -Depth 100) + [Environment]::NewLine)

$verificationJson = (& $PythonPath $verifyScript --result $resultPath --output $verificationPath 2>&1 | Out-String).Trim()
if ($LASTEXITCODE -ne 0) {
    throw "ACP fake-tool verification failed: $verificationJson"
}
$verification = $verificationJson | ConvertFrom-Json
if (-not $verification.valid) {
    throw 'ACP fake-tool verifier returned valid=false.'
}
$result | ConvertTo-Json -Depth 100
