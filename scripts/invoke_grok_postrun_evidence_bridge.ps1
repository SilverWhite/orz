[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$SourceRunDirectory,

    [Parameter(Mandatory = $true)]
    [string]$OutputDirectory,

    [ValidateRange(3, 60)]
    [int]$CommandTimeoutSeconds = 10,

    [string]$PythonPath
)

$ErrorActionPreference = 'Stop'
$sourceRoot = (Resolve-Path -LiteralPath $SourceRunDirectory -ErrorAction Stop).Path
$outputRoot = [System.IO.Path]::GetFullPath($OutputDirectory)
if (Test-Path -LiteralPath $outputRoot) {
    throw "Output directory already exists; refusing to overwrite: $outputRoot"
}
$currentIdentity = [System.Security.Principal.WindowsIdentity]::GetCurrent()
$currentPrincipal = New-Object System.Security.Principal.WindowsPrincipal($currentIdentity)
if (-not $currentPrincipal.IsInRole([System.Security.Principal.WindowsBuiltInRole]::Administrator)) {
    throw 'Administrator token required: post-run Grok commands use a fail-closed all-network firewall rule.'
}
foreach ($command in @('Get-NetFirewallProfile', 'New-NetFirewallRule', 'Remove-NetFirewallRule')) {
    if (-not (Get-Command $command -ErrorAction SilentlyContinue)) {
        throw "Required Windows Firewall command is unavailable: $command"
    }
}

if (-not ('LifPostrunJobObject' -as [type])) {
    Add-Type -TypeDefinition @'
using System;
using System.ComponentModel;
using System.Runtime.InteropServices;

public static class LifPostrunJobObject
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

function Assert-PathInside {
    param(
        [Parameter(Mandatory = $true)][string]$Root,
        [Parameter(Mandatory = $true)][string]$Path
    )
    $rootPath = [System.IO.Path]::GetFullPath($Root).TrimEnd('\')
    $candidate = [System.IO.Path]::GetFullPath($Path)
    if (
        -not $candidate.Equals($rootPath, [System.StringComparison]::OrdinalIgnoreCase) -and
        -not $candidate.StartsWith($rootPath + '\', [System.StringComparison]::OrdinalIgnoreCase)
    ) {
        throw "Path escaped expected root: $candidate"
    }
    return $candidate
}

function New-CleanEnvironment {
    param(
        [Parameter(Mandatory = $true)][string]$ProfileRoot,
        [Parameter(Mandatory = $true)][string]$GrokHome,
        [Parameter(Mandatory = $true)][string]$TempRoot
    )
    return @{
        'SystemRoot' = $env:SystemRoot
        'WINDIR' = $env:WINDIR
        'COMSPEC' = $env:COMSPEC
        'PATH' = $env:PATH
        'PATHEXT' = $env:PATHEXT
        'OS' = $env:OS
        'PROCESSOR_ARCHITECTURE' = $env:PROCESSOR_ARCHITECTURE
        'NUMBER_OF_PROCESSORS' = $env:NUMBER_OF_PROCESSORS
        'HOME' = $ProfileRoot
        'USERPROFILE' = $ProfileRoot
        'APPDATA' = $ProfileRoot
        'LOCALAPPDATA' = $ProfileRoot
        'GROK_HOME' = $GrokHome
        'TEMP' = $TempRoot
        'TMP' = $TempRoot
        'HTTP_PROXY' = 'http://127.0.0.1:1'
        'HTTPS_PROXY' = 'http://127.0.0.1:1'
        'ALL_PROXY' = 'http://127.0.0.1:1'
        'NO_PROXY' = '127.0.0.1,localhost'
    }
}

function Invoke-ControlledCommand {
    param(
        [Parameter(Mandatory = $true)][ValidateSet('trace', 'export')][string]$Kind,
        [Parameter(Mandatory = $true)][string]$Executable,
        [Parameter(Mandatory = $true)][string[]]$Arguments,
        [Parameter(Mandatory = $true)][string]$WorkingDirectory,
        [Parameter(Mandatory = $true)][hashtable]$Environment,
        [Parameter(Mandatory = $true)][string]$StdoutPath,
        [Parameter(Mandatory = $true)][string]$StderrPath,
        [Parameter(Mandatory = $true)][string]$StatusPath,
        [Parameter(Mandatory = $true)][string]$ExpectedArtifactPath,
        [Parameter(Mandatory = $true)][string]$BinarySha256,
        [Parameter(Mandatory = $true)][string]$SessionId
    )
    $info = New-Object System.Diagnostics.ProcessStartInfo
    $info.FileName = $Executable
    $info.Arguments = Join-NativeArguments -Values $Arguments
    $info.WorkingDirectory = $WorkingDirectory
    $info.UseShellExecute = $false
    $info.CreateNoWindow = $true
    $info.RedirectStandardOutput = $true
    $info.RedirectStandardError = $true
    $childEnvironment = New-Object 'System.Collections.Generic.Dictionary[string,string]' ([System.StringComparer]::OrdinalIgnoreCase)
    foreach ($name in $Environment.Keys) {
        if ($null -ne $Environment[$name]) {
            $childEnvironment[$name] = [string]$Environment[$name]
        }
    }
    $environmentField = $info.GetType().GetField(
        'environment',
        [System.Reflection.BindingFlags]'Instance,NonPublic'
    )
    if ($null -eq $environmentField) {
        throw 'Could not initialize a clean ProcessStartInfo environment.'
    }
    $environmentField.SetValue($info, $childEnvironment)

    $process = New-Object System.Diagnostics.Process
    $process.StartInfo = $info
    $stopwatch = [System.Diagnostics.Stopwatch]::StartNew()
    if (-not $process.Start()) {
        throw "Failed to start Grok post-run command: $Kind"
    }
    $stdoutTask = $process.StandardOutput.ReadToEndAsync()
    $stderrTask = $process.StandardError.ReadToEndAsync()
    $jobHandle = [IntPtr]::Zero
    $jobCreated = $false
    $jobAssigned = $false
    $jobClosed = $false
    try {
        $jobHandle = [LifPostrunJobObject]::CreateKillOnClose()
        $jobCreated = $true
        [LifPostrunJobObject]::Assign($jobHandle, $process.Handle)
        $jobAssigned = $true
        $timedOut = -not $process.WaitForExit($CommandTimeoutSeconds * 1000)
        [LifPostrunJobObject]::Close($jobHandle)
        $jobClosed = $true
        if ($timedOut -and -not $process.WaitForExit(5000)) {
            Stop-Process -Id $process.Id -Force -ErrorAction Stop
            $process.WaitForExit()
        }
    } finally {
        if (-not $jobClosed -and $jobHandle -ne [IntPtr]::Zero) {
            try {
                [LifPostrunJobObject]::Close($jobHandle)
                $jobClosed = $true
            } catch {}
        }
        if (-not $process.HasExited) {
            Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
            $process.WaitForExit()
        }
        $stopwatch.Stop()
    }
    Write-Utf8Atomic -Path $StdoutPath -Content $stdoutTask.Result
    Write-Utf8Atomic -Path $StderrPath -Content $stderrTask.Result
    $artifactCreated = Test-Path -LiteralPath $ExpectedArtifactPath -PathType Leaf
    $state = if ($timedOut) {
        'timed_out'
    } elseif ($process.ExitCode -eq 0 -and $artifactCreated) {
        'available'
    } else {
        'failed'
    }
    $status = [ordered]@{
        schema_version = '0.1.0'
        status_kind = 'grok-postrun-command'
        command_kind = $Kind
        state = $state
        completed_at = (Get-Date).ToUniversalTime().ToString('o')
        binary_sha256 = $BinarySha256
        session_id = $SessionId
        arguments = $Arguments
        exit_code = [int]$process.ExitCode
        timed_out = $timedOut
        duration_ms = [math]::Round($stopwatch.Elapsed.TotalMilliseconds, 3)
        artifact_created = $artifactCreated
        artifact = if ($artifactCreated) { Get-ArtifactRecord -Path $ExpectedArtifactPath } else { $null }
        stdout = Get-ArtifactRecord -Path $StdoutPath
        stderr = Get-ArtifactRecord -Path $StderrPath
        containment = [ordered]@{
            job_object_created = $jobCreated
            job_object_assigned = $jobAssigned
            kill_on_close = $true
            job_object_closed = $jobClosed
            assignment_race_known = $true
        }
        safety = [ordered]@{
            clean_environment = $true
            fail_closed_proxy = $true
            all_network_firewall_block = $true
            local_only_flag = ($Kind -eq 'trace' -and ($Arguments -contains '--local'))
            debug_capture_disabled = -not ($Arguments -contains '--debug-file')
            credential_present = $false
        }
    }
    Write-Utf8Atomic -Path $StatusPath -Content (($status | ConvertTo-Json -Depth 10) + [Environment]::NewLine)
    return $status
}

$resultPath = Join-Path $sourceRoot 'result.json'
$sourceResult = Get-Content -LiteralPath $resultPath -Raw -Encoding UTF8 | ConvertFrom-Json
if ($sourceResult.valid -ne $true -or $sourceResult.schema_version -notin @('0.2.0', '0.3.0')) {
    throw 'Source run must be a valid trust-gated fake-provider result (schema 0.2.0 or 0.3.0).'
}
$binaryPath = (Resolve-Path -LiteralPath $sourceResult.binary.path -ErrorAction Stop).Path
$binarySha256 = (Get-FileHash -LiteralPath $binaryPath -Algorithm SHA256).Hash.ToLowerInvariant()
if ($binarySha256 -ne $sourceResult.binary.sha256) {
    throw 'Source run binary digest no longer matches the local executable.'
}
$configPath = Assert-PathInside -Root $sourceRoot -Path $sourceResult.artifacts.config.path
$promptPath = Assert-PathInside -Root $sourceRoot -Path $sourceResult.artifacts.prompt.path
$profileRoot = Split-Path -Parent (Split-Path -Parent $configPath)
$grokHome = Split-Path -Parent $configPath
$workspace = Split-Path -Parent $promptPath
$sessionId = $sourceResult.session_id

if (-not $PythonPath) {
    $PythonPath = (Get-Command python -ErrorAction Stop).Source
}
$PythonPath = (Resolve-Path -LiteralPath $PythonPath).Path
$builderPath = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot 'build_grok_event_bridge.py')).Path
$verifierPath = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot 'verify_grok_event_bridge.py')).Path
$schemaRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..\integration\grok')).Path

$commandRoot = Join-Path $outputRoot 'commands'
$tempRoot = Join-Path $outputRoot 'temp'
$bridgeRoot = Join-Path $outputRoot 'bridge'
$postrunReceiptPath = Join-Path $outputRoot 'workspace-trust-receipt.postrun.json'
$traceArtifactPath = Join-Path $outputRoot 'session.trace.tar.gz'
$exportArtifactPath = Join-Path $outputRoot 'session.export.md'
$traceStdoutPath = Join-Path $commandRoot 'trace.stdout.json'
$traceStderrPath = Join-Path $commandRoot 'trace.stderr.log'
$traceStatusPath = Join-Path $commandRoot 'trace-status.json'
$exportStdoutPath = Join-Path $commandRoot 'export.stdout.log'
$exportStderrPath = Join-Path $commandRoot 'export.stderr.log'
$exportStatusPath = Join-Path $commandRoot 'export-status.json'
$failurePath = Join-Path $outputRoot 'failure.json'
$resultOutputPath = Join-Path $outputRoot 'result.json'
$verificationPath = Join-Path $outputRoot 'bridge-verification.json'

New-Item -ItemType Directory -Path $outputRoot,$commandRoot,$tempRoot | Out-Null
$createdFirewallRules = New-Object System.Collections.Generic.List[string]
$firewallRulesRemoved = $false
$traceStatus = $null
$exportStatus = $null
$postrunReceipt = $null

try {
    $receiptArguments = @(
        '-NoProfile', '-ExecutionPolicy', 'Bypass',
        '-File', (Join-Path $PSScriptRoot 'new_grok_workspace_trust_receipt.ps1'),
        '-WorkspacePath', $workspace,
        '-OutputPath', $postrunReceiptPath,
        '-Decision', 'restricted'
    )
    $receiptJson = (& powershell @receiptArguments 2>&1 | Out-String).Trim()
    if ($LASTEXITCODE -ne 0) {
        throw "Post-run workspace trust receipt failed: $receiptJson"
    }
    $postrunReceipt = $receiptJson | ConvertFrom-Json
    if (
        $postrunReceipt.valid -ne $true -or
        $postrunReceipt.discovery.complete -ne $true -or
        $postrunReceipt.discovery.candidate_count -ne 0 -or
        $postrunReceipt.decision.mode -ne 'restricted' -or
        $postrunReceipt.decision.launch_permitted -ne $true -or
        $postrunReceipt.discovery.aggregate_sha256 -ne $sourceResult.workspace_trust.aggregate_sha256 -or
        $postrunReceipt.discovery.scan_policy_sha256 -ne $sourceResult.workspace_trust.scan_policy_sha256
    ) {
        throw 'Post-run workspace trust receipt did not match the source run control surface.'
    }

    $profiles = @(Get-NetFirewallProfile -ErrorAction Stop)
    if ($profiles.Count -eq 0 -or @($profiles | Where-Object { -not $_.Enabled }).Count -gt 0) {
        throw 'All Windows Firewall profiles must be enabled before post-run Grok commands.'
    }
    $ruleName = 'LIFGrokPost-' + [guid]::NewGuid().ToString('N')
    New-NetFirewallRule `
        -Name $ruleName `
        -DisplayName $ruleName `
        -Direction Outbound `
        -Action Block `
        -Enabled True `
        -Profile Any `
        -Program $binaryPath `
        -Protocol Any `
        -RemoteAddress Any | Out-Null
    $createdFirewallRules.Add($ruleName)

    $cleanEnvironment = New-CleanEnvironment -ProfileRoot $profileRoot -GrokHome $grokHome -TempRoot $tempRoot
    $traceStatus = Invoke-ControlledCommand `
        -Kind trace `
        -Executable $binaryPath `
        -Arguments @('trace', '--local', '--json', '--output', $traceArtifactPath, $sessionId) `
        -WorkingDirectory $workspace `
        -Environment $cleanEnvironment `
        -StdoutPath $traceStdoutPath `
        -StderrPath $traceStderrPath `
        -StatusPath $traceStatusPath `
        -ExpectedArtifactPath $traceArtifactPath `
        -BinarySha256 $binarySha256 `
        -SessionId $sessionId
    $exportStatus = Invoke-ControlledCommand `
        -Kind export `
        -Executable $binaryPath `
        -Arguments @('export', $sessionId, $exportArtifactPath) `
        -WorkingDirectory $workspace `
        -Environment $cleanEnvironment `
        -StdoutPath $exportStdoutPath `
        -StderrPath $exportStderrPath `
        -StatusPath $exportStatusPath `
        -ExpectedArtifactPath $exportArtifactPath `
        -BinarySha256 $binarySha256 `
        -SessionId $sessionId
} catch {
    $failure = [ordered]@{
        schema_version = '0.1.0'
        failure_kind = 'grok-postrun-evidence-bridge'
        failed_at = (Get-Date).ToUniversalTime().ToString('o')
        source_run_id = $sourceResult.run_id
        error_type = $_.Exception.GetType().FullName
        message = $_.Exception.Message
        trace_started = ($null -ne $traceStatus)
        export_started = ($null -ne $exportStatus)
    }
    Write-Utf8Atomic -Path $failurePath -Content (($failure | ConvertTo-Json -Depth 8) + [Environment]::NewLine)
    throw
} finally {
    foreach ($rule in $createdFirewallRules) {
        Remove-NetFirewallRule -Name $rule -ErrorAction SilentlyContinue
    }
    $remaining = @($createdFirewallRules | Where-Object { Get-NetFirewallRule -Name $_ -ErrorAction SilentlyContinue })
    $firewallRulesRemoved = $remaining.Count -eq 0
}

if (-not $firewallRulesRemoved) {
    throw 'Post-run all-network firewall rule was not removed.'
}

$builderArguments = @(
    $builderPath,
    '--run-directory', $sourceRoot,
    '--evidence-directory', $outputRoot,
    '--output-directory', $bridgeRoot,
    '--postrun-receipt', $postrunReceiptPath,
    '--trace-status', $traceStatusPath,
    '--export-status', $exportStatusPath
)
$bridgeJson = (& $PythonPath @builderArguments 2>&1 | Out-String).Trim()
if ($LASTEXITCODE -ne 0) {
    throw "Event bridge builder failed: $bridgeJson"
}
$bridgeManifestPath = Join-Path $bridgeRoot 'manifest.json'
$bridgeEventsPath = Join-Path $bridgeRoot 'events.bridge.jsonl'
$bridgeManifest = Get-Content -LiteralPath $bridgeManifestPath -Raw -Encoding UTF8 | ConvertFrom-Json
$verifierArguments = @(
    $verifierPath,
    '--bridge-directory', $bridgeRoot,
    '--schema-directory', $schemaRoot,
    '--output', $verificationPath
)
$verificationJson = (& $PythonPath @verifierArguments 2>&1 | Out-String).Trim()
if ($LASTEXITCODE -ne 0) {
    throw "Event bridge replay verifier failed: $verificationJson"
}
$verification = Get-Content -LiteralPath $verificationPath -Raw -Encoding UTF8 | ConvertFrom-Json

$checks = [ordered]@{
    source_run_valid = ($sourceResult.valid -eq $true)
    postrun_trust_valid = ($postrunReceipt.valid -eq $true)
    workspace_control_aggregate_unchanged = ($postrunReceipt.discovery.aggregate_sha256 -eq $sourceResult.workspace_trust.aggregate_sha256)
    workspace_scan_policy_unchanged = ($postrunReceipt.discovery.scan_policy_sha256 -eq $sourceResult.workspace_trust.scan_policy_sha256)
    all_network_firewall_enforced = ($createdFirewallRules.Count -eq 1)
    firewall_rules_removed = $firewallRulesRemoved
    trace_local_only_requested = ($traceStatus.safety.local_only_flag -eq $true)
    trace_job_closed = ($traceStatus.containment.job_object_closed -eq $true)
    export_job_closed = ($exportStatus.containment.job_object_closed -eq $true)
    debug_capture_disabled = ($traceStatus.safety.debug_capture_disabled -eq $true -and $exportStatus.safety.debug_capture_disabled -eq $true)
    no_credential_injected = ($traceStatus.safety.credential_present -eq $false -and $exportStatus.safety.credential_present -eq $false)
    bridge_valid = ($bridgeManifest.valid -eq $true)
    bridge_verifier_valid = ($verification.valid -eq $true)
}
$valid = -not ($checks.Values -contains $false)
$artifacts = [ordered]@{
    source_result = Get-ArtifactRecord -Path $resultPath
    postrun_trust_receipt = Get-ArtifactRecord -Path $postrunReceiptPath
    trace_status = Get-ArtifactRecord -Path $traceStatusPath
    export_status = Get-ArtifactRecord -Path $exportStatusPath
    bridge_manifest = Get-ArtifactRecord -Path $bridgeManifestPath
    bridge_events = Get-ArtifactRecord -Path $bridgeEventsPath
    bridge_verification = Get-ArtifactRecord -Path $verificationPath
}
if (Test-Path -LiteralPath $traceArtifactPath -PathType Leaf) {
    $artifacts['trace_archive'] = Get-ArtifactRecord -Path $traceArtifactPath
}
if (Test-Path -LiteralPath $exportArtifactPath -PathType Leaf) {
    $artifacts['export_transcript'] = Get-ArtifactRecord -Path $exportArtifactPath
}
$result = [ordered]@{
    schema_version = '0.1.0'
    result_kind = 'grok-postrun-evidence-bridge'
    created_at = (Get-Date).ToUniversalTime().ToString('o')
    source_run_id = $sourceResult.run_id
    session_id = $sessionId
    source_result_schema = $sourceResult.schema_version
    workspace_trust = [ordered]@{
        receipt_id = $postrunReceipt.receipt_id
        candidate_count = [int]$postrunReceipt.discovery.candidate_count
        aggregate_sha256 = $postrunReceipt.discovery.aggregate_sha256
        scan_policy_sha256 = $postrunReceipt.discovery.scan_policy_sha256
    }
    commands = [ordered]@{
        trace = $traceStatus
        export = $exportStatus
    }
    bridge = [ordered]@{
        bridge_id = $bridgeManifest.bridge_id
        completeness = $bridgeManifest.completeness
        event_count = [int]$bridgeManifest.counts.events
    }
    artifacts = $artifacts
    checks = $checks
    valid = $valid
    limitations = @(
        'Trace/export command failure is an observed completeness gap and does not invalidate the preserved source run.',
        'The all-network firewall rule is scoped to the verified Grok executable path; unexpected descendant executables are not covered.',
        'Job assignment occurs immediately after process start and retains a small pre-assignment race.',
        'Session/update/provider-private source files remain plaintext fake-fixture artifacts pending a sealed-private layer.',
        'A valid metadata bridge with partial completeness is not safety or scientific proof.'
    )
}
Write-Utf8Atomic -Path $resultOutputPath -Content (($result | ConvertTo-Json -Depth 18) + [Environment]::NewLine)
$result | ConvertTo-Json -Depth 18
if (-not $valid) {
    exit 2
}
