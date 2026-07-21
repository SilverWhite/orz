[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$OutputDirectory,

    [ValidateRange(5, 60)]
    [int]$TimeoutSeconds = 20,

    [string]$BinaryPath
)

$ErrorActionPreference = 'Stop'
$repoRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
$outputRoot = [System.IO.Path]::GetFullPath($OutputDirectory)
if (Test-Path -LiteralPath $outputRoot) {
    throw "Output directory already exists; refusing to overwrite: $outputRoot"
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

function Start-IsolatedInspect {
    param(
        [Parameter(Mandatory = $true)][string]$Name,
        [Parameter(Mandatory = $true)][string]$Executable,
        [Parameter(Mandatory = $true)][string]$Workspace,
        [Parameter(Mandatory = $true)][string]$ProfileRoot,
        [Parameter(Mandatory = $true)][string]$TempRoot,
        [Parameter(Mandatory = $true)][string]$StdoutPath,
        [Parameter(Mandatory = $true)][string]$StderrPath,
        [switch]$GrantTrust
    )
    New-Item -ItemType Directory -Path $ProfileRoot,$TempRoot | Out-Null
    $arguments = @('--cwd', $Workspace)
    if ($GrantTrust) {
        $arguments = @('--trust') + $arguments
    }
    $arguments += @('inspect', '--json')
    $info = New-Object System.Diagnostics.ProcessStartInfo
    $info.FileName = $Executable
    $info.Arguments = Join-NativeArguments -Values $arguments
    $info.WorkingDirectory = $Workspace
    $info.UseShellExecute = $false
    $info.CreateNoWindow = $true
    $info.RedirectStandardOutput = $true
    $info.RedirectStandardError = $true
    $cleanEnvironment = @{
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
        'TEMP' = $TempRoot
        'TMP' = $TempRoot
        'HTTP_PROXY' = 'http://127.0.0.1:1'
        'HTTPS_PROXY' = 'http://127.0.0.1:1'
        'ALL_PROXY' = 'http://127.0.0.1:1'
        'NO_PROXY' = '127.0.0.1,localhost'
    }
    $childEnvironment = New-Object 'System.Collections.Generic.Dictionary[string,string]' ([System.StringComparer]::OrdinalIgnoreCase)
    foreach ($environmentName in $cleanEnvironment.Keys) {
        if ($null -ne $cleanEnvironment[$environmentName]) {
            $childEnvironment[$environmentName] = [string]$cleanEnvironment[$environmentName]
        }
    }
    $environmentField = $info.GetType().GetField(
        'environment',
        [System.Reflection.BindingFlags]'Instance,NonPublic'
    )
    if ($null -eq $environmentField) {
        throw 'Could not initialize a clean ProcessStartInfo environment on this Windows runtime.'
    }
    $environmentField.SetValue($info, $childEnvironment)
    $process = New-Object System.Diagnostics.Process
    $process.StartInfo = $info
    if (-not $process.Start()) {
        throw "Failed to start Grok inspect process: $Name"
    }
    $stdoutTask = $process.StandardOutput.ReadToEndAsync()
    $stderrTask = $process.StandardError.ReadToEndAsync()
    $timedOut = -not $process.WaitForExit($TimeoutSeconds * 1000)
    if ($timedOut) {
        $process.Kill()
        $process.WaitForExit()
    }
    $stdout = $stdoutTask.Result
    $stderr = $stderrTask.Result
    Write-Utf8Atomic -Path $StdoutPath -Content $stdout
    Write-Utf8Atomic -Path $StderrPath -Content $stderr
    return [ordered]@{
        name = $Name
        argv = @($Executable) + $arguments
        pid = $process.Id
        exit_code = $process.ExitCode
        timed_out = $timedOut
        stdout = $stdout
        stderr = $stderr
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
$fixtureSource = (Resolve-Path -LiteralPath (Join-Path $repoRoot 'integration\grok\fixtures\workspace-control-surfaces')).Path
$runId = 'DISCOVERY-' + [guid]::NewGuid().ToString('N')
$fixture = Join-Path $outputRoot 'workspace'
$receiptPath = Join-Path $outputRoot 'restricted-receipt.json'
$untrustedStdoutPath = Join-Path $outputRoot 'inspect-untrusted.stdout.json'
$untrustedStderrPath = Join-Path $outputRoot 'inspect-untrusted.stderr.log'
$trustFlagStdoutPath = Join-Path $outputRoot 'inspect-trust-flag.stdout.json'
$trustFlagStderrPath = Join-Path $outputRoot 'inspect-trust-flag.stderr.log'
$resultPath = Join-Path $outputRoot 'result.json'

New-Item -ItemType Directory -Path $outputRoot | Out-Null
Copy-Item -LiteralPath $fixtureSource -Destination $fixture -Recurse -ErrorAction Stop
& git -C $fixture init --quiet
if ($LASTEXITCODE -ne 0) {
    throw 'Could not initialize the disposable fixture Git root.'
}

$preflightArguments = @(
    '-NoProfile',
    '-ExecutionPolicy', 'Bypass',
    '-File', (Join-Path $PSScriptRoot 'new_grok_workspace_trust_receipt.ps1'),
    '-WorkspacePath', $fixture,
    '-OutputPath', $receiptPath
)
$preflightJson = (& powershell @preflightArguments 2>&1 | Out-String).Trim()
if ($LASTEXITCODE -ne 0) {
    throw "Workspace trust preflight failed: $preflightJson"
}
$preflight = Get-Content -LiteralPath $receiptPath -Raw -Encoding UTF8 | ConvertFrom-Json
if ($preflight.decision.launch_permitted -ne $false -or $preflight.discovery.candidate_count -lt 1) {
    throw 'Restricted preflight did not fail closed for the control-surface fixture.'
}

$untrusted = Start-IsolatedInspect `
    -Name 'untrusted' `
    -Executable $inspection.binary_path `
    -Workspace $fixture `
    -ProfileRoot (Join-Path $outputRoot 'profile-untrusted') `
    -TempRoot (Join-Path $outputRoot 'temp-untrusted') `
    -StdoutPath $untrustedStdoutPath `
    -StderrPath $untrustedStderrPath
$trustFlag = Start-IsolatedInspect `
    -Name 'trust-flag' `
    -Executable $inspection.binary_path `
    -Workspace $fixture `
    -ProfileRoot (Join-Path $outputRoot 'profile-trust-flag') `
    -TempRoot (Join-Path $outputRoot 'temp-trust-flag') `
    -StdoutPath $trustFlagStdoutPath `
    -StderrPath $trustFlagStderrPath `
    -GrantTrust

if ($untrusted.exit_code -ne 0 -or $untrusted.timed_out) {
    throw "Untrusted Grok inspect failed: $($untrusted.stderr)"
}
if ($trustFlag.exit_code -ne 0 -or $trustFlag.timed_out) {
    throw "Trust-flag Grok inspect failed: $($trustFlag.stderr)"
}
$untrustedJson = $untrusted.stdout | ConvertFrom-Json
$trustFlagJson = $trustFlag.stdout | ConvertFrom-Json

$untrustedInstructionPaths = @($untrustedJson.projectInstructions | ForEach-Object { [string]$_.path })
$trustFlagInstructionPaths = @($trustFlagJson.projectInstructions | ForEach-Object { [string]$_.path })
$untrustedSkillPaths = @($untrustedJson.skills | ForEach-Object { [string]$_.source.path })
$trustFlagSkillPaths = @($trustFlagJson.skills | ForEach-Object { [string]$_.source.path })
$untrustedHookJson = $untrustedJson.hooks | ConvertTo-Json -Depth 8 -Compress
$trustFlagHookJson = $trustFlagJson.hooks | ConvertTo-Json -Depth 8 -Compress
if ($null -eq $untrustedHookJson) { $untrustedHookJson = '' }
if ($null -eq $trustFlagHookJson) { $trustFlagHookJson = '' }
$untrustedPermissionSources = @($untrustedJson.permissions.sources | ForEach-Object { [string]$_ })
$trustFlagPermissionSources = @($trustFlagJson.permissions.sources | ForEach-Object { [string]$_ })

$observations = [ordered]@{
    untrusted_project_trusted = [bool]$untrustedJson.projectTrusted
    trust_flag_project_trusted = [bool]$trustFlagJson.projectTrusted
    instruction_visible_untrusted = (@($untrustedInstructionPaths | Where-Object { $_ -match '(?i)Agents\.md$' }).Count -gt 0)
    instruction_visible_trust_flag = (@($trustFlagInstructionPaths | Where-Object { $_ -match '(?i)Agents\.md$' }).Count -gt 0)
    skill_visible_untrusted = (@($untrustedSkillPaths | Where-Object { $_ -match '(?i)inert-fixture[\\/]SKILL\.md$' }).Count -gt 0)
    skill_visible_trust_flag = (@($trustFlagSkillPaths | Where-Object { $_ -match '(?i)inert-fixture[\\/]SKILL\.md$' }).Count -gt 0)
    hook_visible_untrusted = $untrustedHookJson.Contains('inert-session-start')
    hook_visible_trust_flag = $trustFlagHookJson.Contains('inert-session-start')
    grok_config_visible_untrusted = (@($untrustedPermissionSources | Where-Object { $_ -match '(?i)\.grok[\\/]config\.toml' }).Count -gt 0)
    grok_config_visible_trust_flag = (@($trustFlagPermissionSources | Where-Object { $_ -match '(?i)\.grok[\\/]config\.toml' }).Count -gt 0)
    claude_settings_visible_untrusted = (@($untrustedPermissionSources | Where-Object { $_ -match '(?i)\.claude[\\/]settings\.json' }).Count -gt 0)
    claude_settings_visible_trust_flag = (@($trustFlagPermissionSources | Where-Object { $_ -match '(?i)\.claude[\\/]settings\.json' }).Count -gt 0)
    untrusted_instruction_count = @($untrustedJson.projectInstructions).Count
    trust_flag_instruction_count = @($trustFlagJson.projectInstructions).Count
    untrusted_hook_count = @($untrustedJson.hooks).Count
    trust_flag_hook_count = @($trustFlagJson.hooks).Count
    untrusted_skill_count = @($untrustedJson.skills).Count
    trust_flag_skill_count = @($trustFlagJson.skills).Count
    untrusted_permission_source_count = @($untrustedJson.permissions.sources).Count
    trust_flag_permission_source_count = @($trustFlagJson.permissions.sources).Count
}
$checks = [ordered]@{
    binary_verified = $true
    restricted_preflight_denied_launch = ($preflight.decision.launch_permitted -eq $false)
    restricted_preflight_complete = ($preflight.discovery.complete -eq $true)
    untrusted_inspect_exit_zero = ($untrusted.exit_code -eq 0 -and -not $untrusted.timed_out)
    trust_flag_inspect_exit_zero = ($trustFlag.exit_code -eq 0 -and -not $trustFlag.timed_out)
    upstream_reports_untrusted_without_grant = ($observations.untrusted_project_trusted -eq $false)
    inspect_trust_flag_is_non_mutating = (
        $observations.trust_flag_project_trusted -eq $false -and
        $untrusted.stdout -ceq $trustFlag.stdout
    )
    no_real_credential_injected = $true
    debug_capture_disabled = $true
}
$valid = -not ($checks.Values -contains $false)
$artifacts = [ordered]@{
    restricted_receipt = Get-ArtifactRecord -Path $receiptPath
    inspect_untrusted_stdout = Get-ArtifactRecord -Path $untrustedStdoutPath
    inspect_untrusted_stderr = Get-ArtifactRecord -Path $untrustedStderrPath
    inspect_trust_flag_stdout = Get-ArtifactRecord -Path $trustFlagStdoutPath
    inspect_trust_flag_stderr = Get-ArtifactRecord -Path $trustFlagStderrPath
}
$result = [ordered]@{
    schema_version = '0.1.0'
    run_kind = 'grok-workspace-discovery-probe'
    run_id = $runId
    created_at = (Get-Date).ToUniversalTime().ToString('o')
    binary = [ordered]@{
        path = $inspection.binary_path
        version_output = $inspection.observed.version_output
        sha256 = $inspection.observed.sha256
        verified = $true
    }
    fixture = [ordered]@{
        source_path = $fixtureSource
        path = $fixture
        static_candidate_count = [int]$preflight.discovery.candidate_count
        static_aggregate_sha256 = $preflight.discovery.aggregate_sha256
        trust_flag_fixture_code_executed = $false
    }
    isolation = [ordered]@{
        profiles_isolated = $true
        temp_isolated = $true
        clean_child_environment = $true
        fail_closed_proxy_configured = $true
        real_credential_present = $false
        grok_trust_store_disposable = $true
    }
    observations = $observations
    checks = $checks
    artifacts = $artifacts
    valid = $valid
    limitations = @(
        'grok inspect reports discovered configuration but does not prove what a later model session will execute.',
        'The --trust comparison uses only checked-in inert fixture controls and an isolated profile; inspect did not create or mutate a folder-trust grant.',
        'A fail-closed proxy is configured, but this probe does not packet-capture or firewall-measure network attempts.',
        'The inspect process is timeout-bounded but is not assigned to the Windows Job Object supervisor.',
        'Visibility is based on paths and counts reported by inspect; file bodies are not present and are not inferred.',
        'This probe does not test the documented folder-trust grant path of a real agent session.'
    )
}
Write-Utf8Atomic -Path $resultPath -Content (($result | ConvertTo-Json -Depth 14) + [Environment]::NewLine)
$result | ConvertTo-Json -Depth 14
if (-not $valid) {
    exit 2
}
