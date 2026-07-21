[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$OutputDirectory,

    [ValidateRange(5, 120)]
    [int]$TimeoutSeconds = 30,

    [string]$BinaryPath,

    [string]$PythonPath
)

$ErrorActionPreference = 'Stop'
$repoRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
$outputRoot = [System.IO.Path]::GetFullPath($OutputDirectory)
if (Test-Path -LiteralPath $outputRoot) {
    throw "Output directory already exists; refusing to overwrite: $outputRoot"
}
if ($outputRoot.Contains('"')) {
    throw 'Output directory may not contain a quote character.'
}
$currentIdentity = [System.Security.Principal.WindowsIdentity]::GetCurrent()
$currentPrincipal = New-Object System.Security.Principal.WindowsPrincipal($currentIdentity)
$administratorRole = [System.Security.Principal.WindowsBuiltInRole]::Administrator
if (-not $currentPrincipal.IsInRole($administratorRole)) {
    throw 'Administrator token required before creating a run: temporary outbound firewall rules are fail-closed.'
}
foreach ($firewallCommand in @('Get-NetFirewallProfile', 'New-NetFirewallRule', 'Remove-NetFirewallRule')) {
    if (-not (Get-Command $firewallCommand -ErrorAction SilentlyContinue)) {
        throw "Required Windows Firewall command is unavailable: $firewallCommand"
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

function Start-RedirectedProcess {
    param(
        [Parameter(Mandatory = $true)][string]$FilePath,
        [Parameter(Mandatory = $true)][string[]]$Arguments,
        [Parameter(Mandatory = $true)][string]$WorkingDirectory,
        [hashtable]$Environment,
        [switch]$CleanEnvironment
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
            $info.EnvironmentVariables[$name] = [string]$Environment[$name]
        }
    }
    $process = New-Object System.Diagnostics.Process
    $process.StartInfo = $info
    if (-not $process.Start()) {
        throw "Failed to start process: $FilePath"
    }
    return [ordered]@{
        process = $process
        stdout_task = $process.StandardOutput.ReadToEndAsync()
        stderr_task = $process.StandardError.ReadToEndAsync()
    }
}

$inspectionArgs = @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', (Join-Path $PSScriptRoot 'inspect_grok_install.ps1'))
if ($BinaryPath) {
    $inspectionArgs += @('-BinaryPath', $BinaryPath)
}
$inspectionJson = (& powershell @inspectionArgs 2>&1 | Out-String).Trim()
if ($LASTEXITCODE -ne 0) {
    throw "Grok binary verification failed: $inspectionJson"
}
$inspection = $inspectionJson | ConvertFrom-Json

if (-not $PythonPath) {
    $pythonCommand = Get-Command python -ErrorAction Stop
    $PythonPath = $pythonCommand.Source
}
$PythonPath = (Resolve-Path -LiteralPath $PythonPath).Path
$providerScript = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot 'fake_deepseek_provider.py')).Path
$promptSource = (Resolve-Path -LiteralPath (Join-Path $repoRoot 'integration\grok\examples\observed-dry-run-prompt.txt')).Path

$runId = 'FAKE-' + [guid]::NewGuid().ToString('N')
$sessionId = [guid]::NewGuid().ToString()
$providerDirectory = Join-Path $outputRoot 'provider'
$profileRoot = Join-Path $outputRoot 'profile'
$grokConfigDirectory = Join-Path $profileRoot '.grok'
$workspace = Join-Path $outputRoot 'workspace'
$tempRoot = Join-Path $outputRoot 'temp'
$promptPath = Join-Path $workspace 'prompt.txt'
$configPath = Join-Path $grokConfigDirectory 'config.toml'
$stdoutPath = Join-Path $outputRoot 'grok.stdout.streaming.jsonl'
$stderrPath = Join-Path $outputRoot 'grok.stderr.log'
$resultPath = Join-Path $outputRoot 'result.json'
$failurePath = Join-Path $outputRoot 'failure.json'

New-Item -ItemType Directory -Path $outputRoot | Out-Null
New-Item -ItemType Directory -Path $grokConfigDirectory | Out-Null
New-Item -ItemType Directory -Path $workspace | Out-Null
New-Item -ItemType Directory -Path $tempRoot | Out-Null
Copy-Item -LiteralPath $promptSource -Destination $promptPath -ErrorAction Stop

$providerHandle = $null
$grokHandle = $null
$createdFirewallRules = New-Object System.Collections.Generic.List[string]
$firewallRulesRemoved = $false
$stopwatch = [System.Diagnostics.Stopwatch]::StartNew()

try {
    $providerHandle = Start-RedirectedProcess `
        -FilePath $PythonPath `
        -Arguments @($providerScript, '--output-directory', $providerDirectory, '--timeout-seconds', ([string]($TimeoutSeconds + 15))) `
        -WorkingDirectory $outputRoot

    $readyPath = Join-Path $providerDirectory 'ready.json'
    $readyDeadline = (Get-Date).AddSeconds(10)
    while (-not (Test-Path -LiteralPath $readyPath)) {
        if ($providerHandle.process.HasExited) {
            $providerError = $providerHandle.stderr_task.Result
            throw "Fake provider exited before ready: $providerError"
        }
        if ((Get-Date) -gt $readyDeadline) {
            throw 'Timed out waiting for fake provider readiness.'
        }
        Start-Sleep -Milliseconds 100
    }
    $ready = Get-Content -LiteralPath $readyPath -Raw -Encoding UTF8 | ConvertFrom-Json
    if (-not $ready.ready -or $ready.host -ne '127.0.0.1' -or $ready.external_bind) {
        throw 'Fake provider readiness record did not prove a loopback-only bind.'
    }

    $config = @"
[cli]
use_leader = false

[model.lif-fake-deepseek]
model = "deepseek-v4-pro"
base_url = "http://127.0.0.1:$($ready.port)"
name = "LIF Fake DeepSeek Loopback"
env_key = "LIF_FAKE_DEEPSEEK_KEY"
api_backend = "chat_completions"
max_completion_tokens = 256
context_window = 4096
"@
    Write-Utf8Atomic -Path $configPath -Content ($config.TrimStart() + [Environment]::NewLine)

    $profiles = @(Get-NetFirewallProfile -ErrorAction Stop)
    if ($profiles.Count -eq 0 -or @($profiles | Where-Object { -not $_.Enabled }).Count -gt 0) {
        throw 'All Windows Firewall profiles must be enabled before the fake-provider run.'
    }
    $rulePrefix = 'LIFGrokFake-' + $runId.Substring(5)
    $firewallSpecs = @(
        @{
            Name = "$rulePrefix-v4-nonloop"
            Remote = @(
                '0.0.0.0/2',
                '64.0.0.0/3',
                '96.0.0.0/4',
                '112.0.0.0/5',
                '120.0.0.0/6',
                '124.0.0.0/7',
                '126.0.0.0/8',
                '128.0.0.0/1'
            )
        },
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
        $createdFirewallRules.Add($spec.Name)
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
        'HTTP_PROXY' = 'http://127.0.0.1:1'
        'HTTPS_PROXY' = 'http://127.0.0.1:1'
        'ALL_PROXY' = 'http://127.0.0.1:1'
        'NO_PROXY' = '127.0.0.1,localhost'
    }
    $grokArguments = @(
        '--cwd', $workspace,
        '--model', 'lif-fake-deepseek',
        '--reasoning-effort', 'high',
        '--max-turns', '1',
        '--output-format', 'streaming-json',
        '--session-id', $sessionId,
        '--no-memory',
        '--no-subagents',
        '--disable-web-search',
        '--permission-mode', 'plan',
        '--sandbox', 'read-only',
        '--tools', 'Read',
        '--verbatim',
        '--prompt-file', $promptPath
    )
    $grokHandle = Start-RedirectedProcess `
        -FilePath $inspection.binary_path `
        -Arguments $grokArguments `
        -WorkingDirectory $workspace `
        -Environment $cleanEnvironment `
        -CleanEnvironment

    $timedOut = -not $grokHandle.process.WaitForExit($TimeoutSeconds * 1000)
    if ($timedOut) {
        Stop-Process -Id $grokHandle.process.Id -Force -ErrorAction Stop
        $grokHandle.process.WaitForExit()
    }
    $grokStdout = $grokHandle.stdout_task.Result
    $grokStderr = $grokHandle.stderr_task.Result
    Write-Utf8Atomic -Path $stdoutPath -Content $grokStdout
    Write-Utf8Atomic -Path $stderrPath -Content $grokStderr

    if (-not $providerHandle.process.WaitForExit(5000)) {
        throw 'Fake provider did not terminate after the Grok request.'
    }
    $providerStdout = $providerHandle.stdout_task.Result
    $providerStderr = $providerHandle.stderr_task.Result
    Write-Utf8Atomic -Path (Join-Path $outputRoot 'provider.stdout.log') -Content $providerStdout
    Write-Utf8Atomic -Path (Join-Path $outputRoot 'provider.stderr.log') -Content $providerStderr
} catch {
    $failure = [ordered]@{
        schema_version = '0.1.0'
        run_kind = 'grok-fake-provider-conformance'
        run_id = $runId
        failed_at = (Get-Date).ToUniversalTime().ToString('o')
        error_type = $_.Exception.GetType().FullName
        message = $_.Exception.Message
        grok_started = ($null -ne $grokHandle)
    }
    Write-Utf8Atomic -Path $failurePath -Content (($failure | ConvertTo-Json -Depth 6) + [Environment]::NewLine)
    throw
} finally {
    if ($grokHandle -and -not $grokHandle.process.HasExited) {
        Stop-Process -Id $grokHandle.process.Id -Force -ErrorAction SilentlyContinue
    }
    if ($providerHandle -and -not $providerHandle.process.HasExited) {
        Stop-Process -Id $providerHandle.process.Id -Force -ErrorAction SilentlyContinue
    }
    foreach ($ruleName in $createdFirewallRules) {
        Remove-NetFirewallRule -Name $ruleName -ErrorAction SilentlyContinue
    }
    $remainingRules = @(
        $createdFirewallRules | Where-Object {
            Get-NetFirewallRule -Name $_ -ErrorAction SilentlyContinue
        }
    )
    $firewallRulesRemoved = $remainingRules.Count -eq 0
    $stopwatch.Stop()
}

if (-not $firewallRulesRemoved) {
    throw 'One or more temporary firewall rules were not removed.'
}

$providerResultPath = Join-Path $providerDirectory 'provider-result.json'
$providerResult = Get-Content -LiteralPath $providerResultPath -Raw -Encoding UTF8 | ConvertFrom-Json
$request = @($providerResult.requests)[0]
$grokStdout = Get-Content -LiteralPath $stdoutPath -Raw -Encoding UTF8
$credentialValueHits = @(
    Get-ChildItem -LiteralPath $outputRoot -Recurse -File |
        Select-String -SimpleMatch 'loopback-fixture-not-a-secret' -ErrorAction SilentlyContinue
)
$checks = [ordered]@{
    grok_exit_zero = ($grokHandle.process.ExitCode -eq 0)
    one_loopback_request = ($providerResult.request_count -eq 1 -and $request.client_ip -eq '127.0.0.1')
    deepseek_path = ($request.path -eq '/chat/completions')
    model_match = ($request.model -eq 'deepseek-v4-pro')
    stream_requested = ($request.stream -eq $true)
    authorization_redacted = ($request.authorization_present -eq $true -and $request.authorization_value_recorded -eq $false)
    response_marker_observed = $grokStdout.Contains('LIF_FAKE_PROVIDER_OK')
    no_tool_event_observed = ($grokStdout -notmatch '(?i)"(?:type|event_type)"\s*:\s*"[^"\r\n]*tool')
    credential_value_absent_from_artifacts = ($credentialValueHits.Count -eq 0)
    debug_capture_disabled = (-not (Test-Path -LiteralPath (Join-Path $outputRoot 'grok.debug.log')))
}
$artifacts = [ordered]@{}
foreach ($entry in @(
    @{ Name = 'config'; Path = $configPath },
    @{ Name = 'prompt'; Path = $promptPath },
    @{ Name = 'stdout'; Path = $stdoutPath },
    @{ Name = 'stderr'; Path = $stderrPath },
    @{ Name = 'provider_result'; Path = $providerResultPath },
    @{ Name = 'provider_private'; Path = (Join-Path $providerDirectory 'requests.private.jsonl') }
)) {
    if (Test-Path -LiteralPath $entry.Path -PathType Leaf) {
        $artifacts[$entry.Name] = Get-ArtifactRecord -Path $entry.Path
    }
}
$valid = -not ($checks.Values -contains $false)
$result = [ordered]@{
    schema_version = '0.1.0'
    run_kind = 'grok-fake-provider-conformance'
    run_id = $runId
    session_id = $sessionId
    created_at = (Get-Date).ToUniversalTime().ToString('o')
    binary = [ordered]@{
        path = $inspection.binary_path
        version_output = $inspection.observed.version_output
        sha256 = $inspection.observed.sha256
        verified = $true
    }
    isolation = [ordered]@{
        profile_isolated = $true
        workspace_isolated = $true
        temp_isolated = $true
        provider_host = '127.0.0.1'
        temporary_firewall_enforced = $true
        firewall_rules_removed = $firewallRulesRemoved
        real_credential_present = $false
    }
    process = [ordered]@{
        pid = $grokHandle.process.Id
        exit_code = $grokHandle.process.ExitCode
        timed_out = $timedOut
        duration_ms = [math]::Round($stopwatch.Elapsed.TotalMilliseconds, 3)
    }
    provider = $providerResult
    artifacts = $artifacts
    checks = $checks
    valid = $valid
    limitations = @(
        'The temporary firewall rules apply to the verified Grok executable path; descendant executables are not covered by those program rules.',
        'Absence of a tool event is established only from the captured streaming-json output for this fixed response.',
        'Grok 0.2.106 debug-file capture is disabled because a rejected probe observed the Authorization value in plaintext debug output.',
        'The provider private capture is plaintext and restricted to the checked-in inert prompt; sensitive prompts require the future sealed-private layer.',
        'This validates one fixed SSE response and does not establish real DeepSeek compatibility, correctness, or scientific validity.'
    )
}
Write-Utf8Atomic -Path $resultPath -Content (($result | ConvertTo-Json -Depth 16) + [Environment]::NewLine)
$result | ConvertTo-Json -Depth 16
if (-not $valid) {
    exit 2
}
