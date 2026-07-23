[CmdletBinding()]
param(
    [string]$GrokPath,

    [Parameter(Mandatory = $true)]
    [string]$OutputDirectory,

    [string]$TemporaryWorkspaceParent = 'C:\tmp',

    [ValidateRange(5, 120)]
    [int]$TimeoutSeconds = 20,

    [string]$PythonPath = 'python',

    [string]$ReleaseMetadataPath
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

if ($env:OS -ne 'Windows_NT') {
    throw 'The ACP initialize containment probe is Windows-only.'
}
$identity = [Security.Principal.WindowsIdentity]::GetCurrent()
$principal = New-Object Security.Principal.WindowsPrincipal($identity)
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    throw 'Administrator privileges are required for the temporary outbound firewall rule.'
}
foreach ($command in @('Get-NetFirewallProfile', 'Get-NetFirewallRule', 'New-NetFirewallRule', 'Remove-NetFirewallRule')) {
    if (-not (Get-Command $command -ErrorAction SilentlyContinue)) {
        throw "Required Windows Firewall command is unavailable: $command"
    }
}

if (-not ('LifAcpInitJobObject' -as [type])) {
    Add-Type -TypeDefinition @'
using System;
using System.ComponentModel;
using System.Runtime.InteropServices;

public static class LifAcpInitJobObject
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

function Write-Utf8Atomic {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][AllowEmptyString()][string]$Content
    )
    if (Test-Path -LiteralPath $Path) {
        throw "Refusing to overwrite output: $Path"
    }
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
    $resolved = (Resolve-Path -LiteralPath $Path -ErrorAction Stop).Path
    $item = Get-Item -LiteralPath $resolved -Force
    return [ordered]@{
        path = $resolved
        bytes = [long]$item.Length
        sha256 = (Get-FileHash -LiteralPath $resolved -Algorithm SHA256).Hash.ToLowerInvariant()
    }
}

function Join-NativeArguments {
    param([Parameter(Mandatory = $true)][string[]]$Values)
    $quoted = foreach ($value in $Values) {
        if ($value -notmatch '[\s"]') {
            $value
        } else {
            '"' + (($value -replace '(\\*)"', '$1$1\"') -replace '(\\+)$', '$1$1') + '"'
        }
    }
    return ($quoted -join ' ')
}

function Add-LeafPaths {
    param(
        [AllowNull()]$Value,
        [Parameter(Mandatory = $true)][string]$Prefix,
        [Parameter(Mandatory = $true)][AllowEmptyCollection()][System.Collections.Generic.List[string]]$Output
    )
    if ($null -eq $Value) {
        $Output.Add($Prefix)
        return
    }
    if ($Value -is [System.Collections.IDictionary]) {
        foreach ($key in @($Value.Keys | Sort-Object)) {
            Add-LeafPaths -Value $Value[$key] -Prefix ($Prefix + '.' + $key) -Output $Output
        }
        return
    }
    if ($Value -is [System.Management.Automation.PSCustomObject]) {
        foreach ($property in @($Value.PSObject.Properties | Sort-Object Name)) {
            Add-LeafPaths -Value $property.Value -Prefix ($Prefix + '.' + $property.Name) -Output $Output
        }
        return
    }
    if ($Value -is [System.Collections.IEnumerable] -and $Value -isnot [string]) {
        $items = @($Value)
        if ($items.Count -eq 0) {
            $Output.Add($Prefix + '[]')
        } else {
            foreach ($item in $items) {
                Add-LeafPaths -Value $item -Prefix ($Prefix + '[]') -Output $Output
            }
        }
        return
    }
    $Output.Add($Prefix)
}

function Get-OptionalProperty {
    param(
        [AllowNull()]$Object,
        [Parameter(Mandatory = $true)][string]$Name
    )
    if ($null -eq $Object) {
        return $null
    }
    $property = $Object.PSObject.Properties[$Name]
    if ($null -eq $property) {
        return $null
    }
    return $property.Value
}

function Test-AllowedInitializeNotification {
    param([AllowNull()]$Value)
    if ($null -eq $Value -or $Value -isnot [System.Management.Automation.PSCustomObject]) {
        return $false
    }
    $names = @($Value.PSObject.Properties.Name | Sort-Object)
    if (($names -join '|') -ne 'jsonrpc|method|params') {
        return $false
    }
    if ($Value.jsonrpc -ne '2.0' -or $Value.method -ne '_x.ai/mcp/servers_updated') {
        return $false
    }
    $params = $Value.params
    if ($null -eq $params -or $params -isnot [System.Management.Automation.PSCustomObject]) {
        return $false
    }
    $paramNames = @($params.PSObject.Properties.Name)
    if ($paramNames.Count -ne 1 -or $paramNames[0] -ne 'mcpServers') {
        return $false
    }
    return @($params.PSObject.Properties['mcpServers'].Value).Count -eq 0
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

function Invoke-TrustReceipt {
    param(
        [Parameter(Mandatory = $true)][string]$Workspace,
        [Parameter(Mandatory = $true)][string]$ReceiptPath
    )
    $script = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot 'new_grok_workspace_trust_receipt.ps1')).Path
    $arguments = @(
        '-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', $script,
        '-WorkspacePath', $Workspace,
        '-OutputPath', $ReceiptPath,
        '-Decision', 'restricted',
        '-DecisionActor', 'acp-initialize-probe'
    )
    $output = (& powershell @arguments 2>&1 | Out-String).Trim()
    if ($LASTEXITCODE -ne 0) {
        throw "Workspace trust receipt failed: $output"
    }
    return (Get-Content -LiteralPath $ReceiptPath -Raw -Encoding UTF8 | ConvertFrom-Json)
}

$repoRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
$lockPath = if ($ReleaseMetadataPath) {
    (Resolve-Path -LiteralPath $ReleaseMetadataPath -ErrorAction Stop).Path
} else {
    Join-Path $repoRoot 'upstream\grok-build.lock.json'
}
$lock = Get-Content -LiteralPath $lockPath -Raw -Encoding UTF8 | ConvertFrom-Json
if ($null -eq $lock.binary_release) {
    throw "Release metadata does not contain binary_release: $lockPath"
}
$selectedGrokPath = if ([string]::IsNullOrWhiteSpace($GrokPath)) {
    Join-Path $repoRoot ([string]$lock.binary_release.installed_path -replace '/', '\')
} else {
    $GrokPath
}
$binaryPath = (Resolve-Path -LiteralPath $selectedGrokPath -ErrorAction Stop).Path
$binaryItem = Get-Item -LiteralPath $binaryPath -Force
$binarySha256 = (Get-FileHash -LiteralPath $binaryPath -Algorithm SHA256).Hash.ToLowerInvariant()
$lockedSha256 = [string]$lock.binary_release.sha256
if ($binarySha256 -ne $lockedSha256 -or $binaryItem.Length -ne [long]$lock.binary_release.bytes) {
    throw 'Grok binary does not match the selected release metadata digest/size.'
}
$signature = Get-AuthenticodeSignature -LiteralPath $binaryPath
if ($signature.Status -ne 'Valid' -or $signature.SignerCertificate.Subject -notmatch 'CN=X\.AI LLC(?:,|$)') {
    throw 'Grok binary Authenticode signature is not the selected X.AI LLC release signature.'
}

$outputRoot = [System.IO.Path]::GetFullPath($OutputDirectory)
if (Test-Path -LiteralPath $outputRoot) {
    throw "Output directory already exists; refusing to overwrite: $outputRoot"
}
$outputParent = Split-Path -Parent $outputRoot
if (-not (Test-Path -LiteralPath $outputParent -PathType Container)) {
    throw "Output parent does not exist: $outputParent"
}
$temporaryParent = (Resolve-Path -LiteralPath $TemporaryWorkspaceParent -ErrorAction Stop).Path
$probeId = 'ACPINIT-' + [guid]::NewGuid().ToString('N')
$workspace = Join-Path $temporaryParent ('lif-acp-init-' + $probeId.Substring(8))
New-Item -ItemType Directory -Path $outputRoot -ErrorAction Stop | Out-Null
New-Item -ItemType Directory -Path $workspace -ErrorAction Stop | Out-Null

$profileRoot = Join-Path $outputRoot 'profile'
$grokHome = Join-Path $profileRoot '.grok'
$tempRoot = Join-Path $outputRoot 'temp'
foreach ($directory in @($profileRoot, $grokHome, $tempRoot)) {
    New-Item -ItemType Directory -Path $directory -ErrorAction Stop | Out-Null
}
$requestPath = Join-Path $outputRoot 'request.json'
$responsePath = Join-Path $outputRoot 'response.json'
$stdoutPath = Join-Path $outputRoot 'stdout.jsonl'
$stderrPath = Join-Path $outputRoot 'stderr.txt'
$preflightPath = Join-Path $outputRoot 'workspace-trust.preflight.json'
$postrunPath = Join-Path $outputRoot 'workspace-trust.postrun.json'
$resultPath = Join-Path $outputRoot 'result.json'
$verificationPath = Join-Path $outputRoot 'verification.json'
$failurePath = Join-Path $outputRoot 'failure.json'

$requestId = 'lif-acp-init-1'
$request = [ordered]@{
    jsonrpc = '2.0'
    id = $requestId
    method = 'initialize'
    params = [ordered]@{
        protocolVersion = 1
        clientCapabilities = [ordered]@{}
        clientInfo = [ordered]@{
            name = 'lif-acp-capability-probe'
            title = 'LIF ACP Capability Probe'
            version = '0.1.0'
        }
    }
}
$requestPretty = ($request | ConvertTo-Json -Depth 10) + [Environment]::NewLine
$requestLine = $request | ConvertTo-Json -Depth 10 -Compress
Write-Utf8Atomic -Path $requestPath -Content $requestPretty

$startedAt = (Get-Date).ToUniversalTime().ToString('o')
$preflight = $null
$postrun = $null
$firewallRuleName = 'LIFGrokACPInit-' + [guid]::NewGuid().ToString('N')
$firewallCreated = $false
$firewallRemoved = $false
$remainingRuleCount = 0
$jobHandle = [IntPtr]::Zero
$jobCreated = $false
$jobAssigned = $false
$jobClosed = $false
$process = $null
$timedOut = $false
$terminalState = 'unknown'
$responseState = 'eof'
$responseObject = $null
$responseLine = $null
$stdoutLines = New-Object System.Collections.Generic.List[string]
$parseFailures = 0
$unexpectedLines = 0
$allowedNotifications = New-Object System.Collections.Generic.List[string]
$cleanEnvironment = New-CleanEnvironment -ProfileRoot $profileRoot -GrokHome $grokHome -TempRoot $tempRoot
$stopwatch = [System.Diagnostics.Stopwatch]::StartNew()

try {
    $preflight = Invoke-TrustReceipt -Workspace $workspace -ReceiptPath $preflightPath
    if (
        $preflight.valid -ne $true -or
        $preflight.discovery.complete -ne $true -or
        $preflight.discovery.candidate_count -ne 0 -or
        $preflight.decision.mode -ne 'restricted' -or
        $preflight.decision.launch_permitted -ne $true
    ) {
        throw 'Preflight did not establish an empty restricted workspace.'
    }

    $profiles = @(Get-NetFirewallProfile -ErrorAction Stop)
    if ($profiles.Count -eq 0 -or @($profiles | Where-Object { -not $_.Enabled }).Count -gt 0) {
        throw 'All Windows Firewall profiles must be enabled before the ACP probe.'
    }
    New-NetFirewallRule `
        -Name $firewallRuleName `
        -DisplayName $firewallRuleName `
        -Direction Outbound `
        -Action Block `
        -Enabled True `
        -Profile Any `
        -Program $binaryPath `
        -Protocol Any `
        -RemoteAddress Any | Out-Null
    $firewallCreated = $true

    $startInfo = New-Object System.Diagnostics.ProcessStartInfo
    $startInfo.FileName = $binaryPath
    $startInfo.Arguments = Join-NativeArguments -Values @('agent', 'stdio')
    $startInfo.WorkingDirectory = $workspace
    $startInfo.UseShellExecute = $false
    $startInfo.CreateNoWindow = $true
    $startInfo.RedirectStandardInput = $true
    $startInfo.RedirectStandardOutput = $true
    $startInfo.RedirectStandardError = $true
    $startInfo.EnvironmentVariables.Clear()
    foreach ($name in $cleanEnvironment.Keys) {
        if ($null -ne $cleanEnvironment[$name]) {
            $startInfo.EnvironmentVariables[$name] = [string]$cleanEnvironment[$name]
        }
    }
    if ($startInfo.EnvironmentVariables.Count -ne @($cleanEnvironment.Keys | Where-Object { $null -ne $cleanEnvironment[$_] }).Count) {
        throw 'Could not initialize the complete clean ProcessStartInfo environment.'
    }

    $process = New-Object System.Diagnostics.Process
    $process.StartInfo = $startInfo
    if (-not $process.Start()) {
        $terminalState = 'failed_to_start'
        throw 'Failed to start Grok ACP stdio process.'
    }
    $stderrTask = $process.StandardError.ReadToEndAsync()
    $jobHandle = [LifAcpInitJobObject]::CreateKillOnClose()
    $jobCreated = $true
    [LifAcpInitJobObject]::Assign($jobHandle, $process.Handle)
    $jobAssigned = $true

    $process.StandardInput.WriteLine($requestLine)
    $process.StandardInput.Flush()
    $deadline = [DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
    while ($null -eq $responseObject -and [DateTime]::UtcNow -lt $deadline) {
        $remainingMs = [math]::Max(1, [int]($deadline - [DateTime]::UtcNow).TotalMilliseconds)
        $lineTask = $process.StandardOutput.ReadLineAsync()
        if (-not $lineTask.Wait($remainingMs)) {
            $timedOut = $true
            $responseState = 'timeout'
            break
        }
        $line = $lineTask.Result
        if ($null -eq $line) {
            $responseState = 'eof'
            break
        }
        $stdoutLines.Add($line)
        try {
            $candidate = $line | ConvertFrom-Json -ErrorAction Stop
            if ([string]$candidate.id -eq $requestId) {
                $responseObject = $candidate
                $responseLine = $line
                $responseState = if ($null -ne (Get-OptionalProperty -Object $candidate -Name 'error')) { 'rpc_error' } else { 'observed' }
                break
            }
            if (Test-AllowedInitializeNotification -Value $candidate) {
                $allowedNotifications.Add([string]$candidate.method)
            } else {
                $unexpectedLines += 1
            }
        } catch {
            $parseFailures += 1
            $unexpectedLines += 1
        }
    }

    $process.StandardInput.Close()
    $remainingStdoutTask = $null
    if ($null -ne $responseObject) {
        $remainingStdoutTask = $process.StandardOutput.ReadToEndAsync()
    }
    if ($process.WaitForExit(2000)) {
        $terminalState = 'exited'
    } else {
        [LifAcpInitJobObject]::Close($jobHandle)
        $jobClosed = $true
        $terminalState = if ($timedOut) { 'timed_out' } else { 'terminated_after_response' }
        if (-not $process.WaitForExit(5000)) {
            Stop-Process -Id $process.Id -Force -ErrorAction Stop
            $process.WaitForExit()
        }
    }
    if ($null -ne $remainingStdoutTask) {
        $remainingText = $remainingStdoutTask.Result
        if (-not [string]::IsNullOrWhiteSpace($remainingText)) {
            foreach ($line in @($remainingText -split "`r?`n" | Where-Object { $_.Length -gt 0 })) {
                $stdoutLines.Add($line)
                try {
                    $candidate = $line | ConvertFrom-Json -ErrorAction Stop
                    if (Test-AllowedInitializeNotification -Value $candidate) {
                        $allowedNotifications.Add([string]$candidate.method)
                    } else {
                        $unexpectedLines += 1
                    }
                } catch {
                    $parseFailures += 1
                    $unexpectedLines += 1
                }
            }
        }
    }
    $stderrText = $stderrTask.Result
    if (-not $jobClosed -and $jobHandle -ne [IntPtr]::Zero) {
        [LifAcpInitJobObject]::Close($jobHandle)
        $jobClosed = $true
    }
    Write-Utf8Atomic -Path $stdoutPath -Content (($stdoutLines -join [Environment]::NewLine) + $(if ($stdoutLines.Count) { [Environment]::NewLine } else { '' }))
    Write-Utf8Atomic -Path $stderrPath -Content $stderrText
    if ($null -ne $responseObject) {
        Write-Utf8Atomic -Path $responsePath -Content (($responseObject | ConvertTo-Json -Depth 50) + [Environment]::NewLine)
    }

    $postrun = Invoke-TrustReceipt `
        -Workspace $workspace `
        -ReceiptPath $postrunPath
} catch {
    $failure = [ordered]@{
        schema_version = '0.1.0'
        failure_kind = 'grok-acp-initialize-probe'
        failed_at = (Get-Date).ToUniversalTime().ToString('o')
        probe_id = $probeId
        error_type = $_.Exception.GetType().FullName
        message = $_.Exception.Message
        firewall_created = $firewallCreated
        job_created = $jobCreated
        job_assigned = $jobAssigned
        request_written = (Test-Path -LiteralPath $requestPath)
        response_observed = ($null -ne $responseObject)
    }
    if (-not (Test-Path -LiteralPath $failurePath)) {
        Write-Utf8Atomic -Path $failurePath -Content (($failure | ConvertTo-Json -Depth 10) + [Environment]::NewLine)
    }
    throw
} finally {
    if ($null -ne $process -and -not $process.HasExited) {
        if (-not $jobClosed -and $jobHandle -ne [IntPtr]::Zero) {
            try {
                [LifAcpInitJobObject]::Close($jobHandle)
                $jobClosed = $true
            } catch {}
        }
        if (-not $process.WaitForExit(5000)) {
            Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
            $process.WaitForExit()
        }
    }
    if (-not $jobClosed -and $jobHandle -ne [IntPtr]::Zero) {
        try {
            [LifAcpInitJobObject]::Close($jobHandle)
            $jobClosed = $true
        } catch {}
    }
    if ($firewallCreated) {
        Remove-NetFirewallRule -Name $firewallRuleName -ErrorAction SilentlyContinue
    }
    $remainingRuleCount = @(Get-NetFirewallRule -Name $firewallRuleName -ErrorAction SilentlyContinue).Count
    $firewallRemoved = $remainingRuleCount -eq 0
    $stopwatch.Stop()
}

try {
if (-not $firewallRemoved) {
    throw 'Temporary ACP probe firewall rule was not removed.'
}
if ($null -eq $postrun) {
    throw 'Post-run workspace trust receipt was not produced.'
}

$capabilityPaths = New-Object System.Collections.Generic.List[string]
$metaPaths = New-Object System.Collections.Generic.List[string]
$rpcResult = Get-OptionalProperty -Object $responseObject -Name 'result'
$agentCapabilities = Get-OptionalProperty -Object $rpcResult -Name 'agentCapabilities'
$extensionMeta = Get-OptionalProperty -Object $rpcResult -Name '_meta'
$rpcAgentInfo = Get-OptionalProperty -Object $rpcResult -Name 'agentInfo'
$rpcAuthMethods = Get-OptionalProperty -Object $rpcResult -Name 'authMethods'
$rpcProtocolVersion = Get-OptionalProperty -Object $rpcResult -Name 'protocolVersion'
$rpcError = Get-OptionalProperty -Object $responseObject -Name 'error'
$rpcJsonVersion = Get-OptionalProperty -Object $responseObject -Name 'jsonrpc'
if ($null -ne $agentCapabilities) {
    Add-LeafPaths -Value $agentCapabilities -Prefix 'agentCapabilities' -Output $capabilityPaths
}
if ($null -ne $extensionMeta) {
    Add-LeafPaths -Value $extensionMeta -Prefix '_meta' -Output $metaPaths
}
$capabilityPathArray = @($capabilityPaths | Sort-Object -Unique)
$metaPathArray = @($metaPaths | Sort-Object -Unique)
$authMethodCount = if ($null -ne $rpcAuthMethods) { @($rpcAuthMethods).Count } else { $null }
$agentInfo = [ordered]@{
    name = Get-OptionalProperty -Object $rpcAgentInfo -Name 'name'
    title = Get-OptionalProperty -Object $rpcAgentInfo -Name 'title'
    version = Get-OptionalProperty -Object $rpcAgentInfo -Name 'version'
}

$scanPaths = @($requestPath, $stdoutPath, $stderrPath, $preflightPath, $postrunPath)
if (Test-Path -LiteralPath $responsePath) { $scanPaths += $responsePath }
$forbiddenMarkers = @(
    'DEEPSEEK_API_KEY',
    'XAI_API_KEY',
    'OPENAI_API_KEY',
    'ANTHROPIC_API_KEY',
    'Authorization: Bearer',
    'sk-'
)
$hits = New-Object System.Collections.Generic.List[string]
foreach ($path in $scanPaths) {
    $text = Get-Content -LiteralPath $path -Raw -Encoding UTF8
    if ($null -eq $text) {
        $text = ''
    }
    foreach ($marker in $forbiddenMarkers) {
        if ($text.IndexOf($marker, [System.StringComparison]::OrdinalIgnoreCase) -ge 0) {
            $hits.Add($marker + '@' + [System.IO.Path]::GetFileName($path))
        }
    }
}

$checks = [ordered]@{
    binary_locked = ($binarySha256 -eq $lockedSha256 -and $binaryItem.Length -eq [long]$lock.binary_release.bytes)
    signature_valid = ($signature.Status -eq 'Valid' -and $signature.SignerCertificate.Subject -match 'CN=X\.AI LLC(?:,|$)')
    workspace_restricted_empty_and_stable = (
        $preflight.valid -eq $true -and
        $postrun.valid -eq $true -and
        $preflight.discovery.candidate_count -eq 0 -and
        $postrun.discovery.candidate_count -eq 0 -and
        $preflight.discovery.aggregate_sha256 -eq $postrun.discovery.aggregate_sha256 -and
        $preflight.decision.launch_permitted -eq $true -and
        $postrun.decision.launch_permitted -eq $true
    )
    request_initialize_only = ($request.method -eq 'initialize' -and $request.params.protocolVersion -eq 1)
    response_id_matches = ($null -ne $responseObject -and [string]$responseObject.id -eq $requestId)
    protocol_version_agreed = ($null -ne $rpcResult -and $rpcProtocolVersion -eq 1)
    rpc_result_observed = ($responseState -eq 'observed' -and $null -ne $rpcResult -and $null -eq $rpcError)
    no_unexpected_stdout = ($unexpectedLines -eq 0 -and $parseFailures -eq 0)
    clean_environment = ($startInfo.EnvironmentVariables.Count -eq $cleanEnvironment.Count)
    network_blocked_and_cleaned = ($firewallCreated -and $firewallRemoved -and $remainingRuleCount -eq 0)
    job_contained = ($jobCreated -and $jobAssigned -and $jobClosed)
    leak_scan_clean = ($hits.Count -eq 0)
}
$result = [ordered]@{
    schema_version = '0.1.0'
    probe_kind = 'grok-acp-initialize-no-model'
    probe_id = $probeId
    valid = (@($checks.Values | Where-Object { $_ -ne $true }).Count -eq 0)
    started_at = $startedAt
    completed_at = (Get-Date).ToUniversalTime().ToString('o')
    binary = [ordered]@{
        path = $binaryPath
        bytes = [long]$binaryItem.Length
        sha256 = $binarySha256
        locked_sha256 = $lockedSha256
        version = [string]$lock.binary_release.version
        authenticode_status = [string]$signature.Status
        signer_subject = [string]$signature.SignerCertificate.Subject
    }
    request = [ordered]@{
        artifact = Get-ArtifactRecord -Path $requestPath
        jsonrpc = '2.0'
        id = $requestId
        method = 'initialize'
        protocol_version = 1
        client_capability_paths = @()
        session_or_prompt_requests_sent = 0
    }
    response = [ordered]@{
        state = $responseState
        artifact = if (Test-Path -LiteralPath $responsePath) { Get-ArtifactRecord -Path $responsePath } else { $null }
        jsonrpc = if ($null -ne $rpcJsonVersion) { [string]$rpcJsonVersion } else { $null }
        id_matches = ($null -ne $responseObject -and [string]$responseObject.id -eq $requestId)
        has_result = ($null -ne $rpcResult)
        has_error = ($null -ne $rpcError)
        protocol_version = $rpcProtocolVersion
        agent_capability_paths = $capabilityPathArray
        extension_meta_paths = $metaPathArray
        agent_info = $agentInfo
        auth_method_count = $authMethodCount
        allowed_notification_count = $allowedNotifications.Count
        allowed_notification_methods = @($allowedNotifications | Sort-Object -Unique)
        unexpected_stdout_line_count = $unexpectedLines
    }
    process = [ordered]@{
        exit_code = if ($null -ne $process -and $process.HasExited) { [int]$process.ExitCode } else { $null }
        timed_out = $timedOut
        duration_ms = [math]::Round($stopwatch.Elapsed.TotalMilliseconds, 3)
        terminal_state = $terminalState
        stdout = Get-ArtifactRecord -Path $stdoutPath
        stderr = Get-ArtifactRecord -Path $stderrPath
        containment = [ordered]@{
            job_object_created = $jobCreated
            job_object_assigned = $jobAssigned
            kill_on_close = $true
            job_object_closed = $jobClosed
            assignment_race_known = $true
        }
    }
    workspace_trust = [ordered]@{
        preflight_receipt = Get-ArtifactRecord -Path $preflightPath
        postrun_receipt = Get-ArtifactRecord -Path $postrunPath
        preflight_valid = ($preflight.valid -eq $true)
        postrun_valid = ($postrun.valid -eq $true)
        candidate_count = [int]$preflight.discovery.candidate_count
        aggregate_unchanged = ($preflight.discovery.aggregate_sha256 -eq $postrun.discovery.aggregate_sha256)
    }
    network = [ordered]@{
        all_profiles_enabled = $true
        outbound_block_created = $firewallCreated
        rule_name = $firewallRuleName
        rule_removed = $firewallRemoved
        remaining_rule_count = $remainingRuleCount
    }
    environment = [ordered]@{
        clean = $true
        variable_names = @($cleanEnvironment.Keys | Sort-Object)
        credential_variable_names = @()
        proxy_fail_closed = $true
    }
    leak_scan = [ordered]@{
        scanned_artifacts = @($scanPaths | ForEach-Object { (Resolve-Path -LiteralPath $_).Path })
        forbidden_markers = $forbiddenMarkers
        hit_count = $hits.Count
        hits = @($hits | Sort-Object -Unique)
    }
    checks = $checks
}
Write-Utf8Atomic -Path $resultPath -Content (($result | ConvertTo-Json -Depth 50) + [Environment]::NewLine)

$verifyOutput = (& $PythonPath (Join-Path $PSScriptRoot 'verify_grok_acp_initialize_probe.py') `
    '--result' $resultPath '--output' $verificationPath 2>&1 | Out-String).Trim()
if ($LASTEXITCODE -ne 0) {
    throw "ACP initialize verification failed: $verifyOutput"
}

if (Test-Path -LiteralPath $workspace -PathType Container) {
    $resolvedWorkspace = (Resolve-Path -LiteralPath $workspace).Path
    $resolvedTemporaryParent = (Resolve-Path -LiteralPath $temporaryParent).Path
    if (-not $resolvedWorkspace.StartsWith($resolvedTemporaryParent.TrimEnd('\') + '\', [System.StringComparison]::OrdinalIgnoreCase)) {
        throw 'Refusing to remove temporary workspace outside the configured parent.'
    }
    Remove-Item -LiteralPath $resolvedWorkspace -Recurse -Force
}

[ordered]@{
    probe_id = $probeId
    valid = $result.valid
    response_state = $responseState
    protocol_version = $result.response.protocol_version
    capability_count = $capabilityPathArray.Count
    extension_meta_count = $metaPathArray.Count
    terminal_state = $terminalState
    output_directory = $outputRoot
    verification = $verifyOutput
} | ConvertTo-Json -Depth 10
} catch {
    $workspaceRemovedAfterFailure = $false
    if (Test-Path -LiteralPath $workspace -PathType Container) {
        try {
            $resolvedWorkspace = (Resolve-Path -LiteralPath $workspace).Path
            $resolvedTemporaryParent = (Resolve-Path -LiteralPath $temporaryParent).Path
            if ($resolvedWorkspace.StartsWith($resolvedTemporaryParent.TrimEnd('\') + '\', [System.StringComparison]::OrdinalIgnoreCase)) {
                Remove-Item -LiteralPath $resolvedWorkspace -Recurse -Force
                $workspaceRemovedAfterFailure = -not (Test-Path -LiteralPath $resolvedWorkspace)
            }
        } catch {}
    }
    $failure = [ordered]@{
        schema_version = '0.1.0'
        failure_kind = 'grok-acp-initialize-probe'
        failed_at = (Get-Date).ToUniversalTime().ToString('o')
        failure_stage = 'postprocess_or_verification'
        probe_id = $probeId
        error_type = $_.Exception.GetType().FullName
        message = $_.Exception.Message
        firewall_created = $firewallCreated
        firewall_removed = $firewallRemoved
        remaining_firewall_rule_count = $remainingRuleCount
        job_created = $jobCreated
        job_assigned = $jobAssigned
        request_written = (Test-Path -LiteralPath $requestPath)
        response_observed = ($null -ne $responseObject)
        postrun_receipt_written = (Test-Path -LiteralPath $postrunPath)
        result_written = (Test-Path -LiteralPath $resultPath)
        verification_written = (Test-Path -LiteralPath $verificationPath)
        workspace_removed_after_failure = $workspaceRemovedAfterFailure
    }
    if (-not (Test-Path -LiteralPath $failurePath)) {
        Write-Utf8Atomic -Path $failurePath -Content (($failure | ConvertTo-Json -Depth 10) + [Environment]::NewLine)
    }
    throw
}
