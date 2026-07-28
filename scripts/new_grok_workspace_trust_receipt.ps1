[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$WorkspacePath,

    [Parameter(Mandatory = $true)]
    [string]$OutputPath,

    [ValidateSet('restricted', 'trusted', 'denied')]
    [string]$Decision = 'restricted',

    [string]$ProjectRootPath,

    [string]$DecisionActor = 'sidecar-default',

    [string]$ExpectedAggregateSha256,

    [ValidateRange(1, 20000)]
    [int]$MaxCandidateFiles = 5000,

    [ValidateRange(1, 2147483647)]
    [long]$MaxCandidateBytes = 268435456
)

$ErrorActionPreference = 'Stop'

function Get-TextSha256 {
    param([Parameter(Mandatory = $true)][AllowEmptyString()][string]$Text)
    $encoding = New-Object System.Text.UTF8Encoding($false)
    $bytes = $encoding.GetBytes($Text)
    $algorithm = [System.Security.Cryptography.SHA256]::Create()
    try {
        return (($algorithm.ComputeHash($bytes) | ForEach-Object { $_.ToString('x2') }) -join '')
    } finally {
        $algorithm.Dispose()
    }
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

function Find-ProjectRoot {
    param([Parameter(Mandatory = $true)][string]$StartPath)
    $cursor = Get-Item -LiteralPath $StartPath -Force -ErrorAction Stop
    while ($null -ne $cursor) {
        if (Test-Path -LiteralPath (Join-Path $cursor.FullName '.git')) {
            return $cursor.FullName
        }
        $cursor = $cursor.Parent
    }
    return $StartPath
}

function Get-RelativeControlPath {
    param(
        [Parameter(Mandatory = $true)][string]$Root,
        [Parameter(Mandatory = $true)][string]$FullPath
    )
    $normalizedRoot = [System.IO.Path]::GetFullPath($Root).TrimEnd([char[]]@('\', '/'))
    $normalizedFullPath = [System.IO.Path]::GetFullPath($FullPath)
    if ($normalizedFullPath.Equals($normalizedRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
        return '.'
    }
    $rootForUri = $normalizedRoot + [System.IO.Path]::DirectorySeparatorChar
    $relativeUri = ([uri]$rootForUri).MakeRelativeUri([uri]$normalizedFullPath)
    $relative = [System.Uri]::UnescapeDataString($relativeUri.ToString())
    if (
        $relativeUri.IsAbsoluteUri -or
        $relative -eq '..' -or
        $relative.StartsWith('../', [System.StringComparison]::Ordinal) -or
        $relative.StartsWith('..\', [System.StringComparison]::Ordinal)
    ) {
        throw "Control path escaped project root: $FullPath"
    }
    return $relative.Replace('\', '/')
}

function Test-PathContainedOrEqual {
    param(
        [Parameter(Mandatory = $true)][string]$Root,
        [Parameter(Mandatory = $true)][string]$Child
    )
    $normalizedRoot = [System.IO.Path]::GetFullPath($Root).TrimEnd([char[]]@('\', '/'))
    $normalizedChild = [System.IO.Path]::GetFullPath($Child)
    if ($normalizedChild.Equals($normalizedRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
        return $true
    }
    $rootForUri = $normalizedRoot + [System.IO.Path]::DirectorySeparatorChar
    $relativeUri = ([uri]$rootForUri).MakeRelativeUri([uri]$normalizedChild)
    $relative = [System.Uri]::UnescapeDataString($relativeUri.ToString())
    return (
        -not $relativeUri.IsAbsoluteUri -and
        $relative -ne '..' -and
        -not $relative.StartsWith('../', [System.StringComparison]::Ordinal) -and
        -not $relative.StartsWith('..\', [System.StringComparison]::Ordinal)
    )
}

function ConvertTo-NormalizedPathForComparison {
    param([Parameter(Mandatory = $true)][string]$Path)
    return [System.IO.Path]::GetFullPath($Path).TrimEnd([char[]]@('\', '/'))
}

$workspace = (Resolve-Path -LiteralPath $WorkspacePath -ErrorAction Stop).Path
$workspaceItem = Get-Item -LiteralPath $workspace -Force -ErrorAction Stop
if (-not $workspaceItem.PSIsContainer) {
    throw "Workspace must be a directory: $workspace"
}
$workspace = $workspaceItem.FullName
$output = [System.IO.Path]::GetFullPath($OutputPath)
if (Test-Path -LiteralPath $output) {
    throw "Output already exists; refusing to overwrite: $output"
}
$outputParent = Split-Path -Parent $output
if (-not (Test-Path -LiteralPath $outputParent -PathType Container)) {
    throw "Output parent directory must already exist: $outputParent"
}

if ([string]::IsNullOrWhiteSpace($ProjectRootPath)) {
    $projectRoot = [System.IO.Path]::GetFullPath((Find-ProjectRoot -StartPath $workspace))
} else {
    $projectRoot = (Resolve-Path -LiteralPath $ProjectRootPath -ErrorAction Stop).Path
    $projectRootItem = Get-Item -LiteralPath $projectRoot -Force -ErrorAction Stop
    if (-not $projectRootItem.PSIsContainer) {
        throw "Project root must be a directory: $projectRoot"
    }
    $projectRoot = $projectRootItem.FullName
}
if (-not (Test-PathContainedOrEqual -Root $projectRoot -Child $workspace)) {
    throw 'Workspace is not contained by the discovered project root.'
}

$scopeDirectories = New-Object System.Collections.Generic.List[string]
$scopeCursor = Get-Item -LiteralPath $workspace -Force
$normalizedProjectRoot = ConvertTo-NormalizedPathForComparison $projectRoot
while ($null -ne $scopeCursor) {
    $scopeDirectories.Add($scopeCursor.FullName)
    $normalizedScopeCursor = ConvertTo-NormalizedPathForComparison $scopeCursor.FullName
    if ($normalizedScopeCursor.Equals($normalizedProjectRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
        break
    }
    $scopeCursor = $scopeCursor.Parent
}
$lastScopeDirectory = ConvertTo-NormalizedPathForComparison $scopeDirectories.Item($scopeDirectories.Count - 1)
if (-not $lastScopeDirectory.Equals($normalizedProjectRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw 'Could not build project-root-to-workspace discovery scope.'
}
$scopeDirectories = @($scopeDirectories.ToArray())
[array]::Reverse($scopeDirectories)

$scanPolicyLines = @(
    'policy=grok-workspace-control-scan-v0.1',
    'instruction-files=Agents.md,Claude.md,CLAUDE.md,CLAUDE.local.md,AGENTS.md',
    'rules=.grok/rules/*.md,.claude/rules/*.md,.cursor/rules/*.md',
    'project-config=.grok/config.toml,.claude/settings.json,.claude/settings.local.json',
    'project-code=.grok/hooks/**,.grok/plugins/**,.grok/skills/**,.grok/agents/**',
    'external-process=.grok/lsp.json,.cursor/hooks.json,.cursor/mcp.json',
    'compat-content=.claude/skills/**,.cursor/skills/**',
    'reparse-policy=record-and-do-not-follow',
    'gitignore-policy=conservative-ignore-status-not-applied'
)
$scanPolicySha256 = Get-TextSha256 -Text (($scanPolicyLines -join "`n") + "`n")

$script:candidates = New-Object System.Collections.Generic.List[object]
$script:seen = @{}
$script:scanComplete = $true
$script:stopScan = $false
$script:totalBytes = [long]0
$script:limitReasons = New-Object System.Collections.Generic.List[string]

function Stop-ControlScan {
    param([Parameter(Mandatory = $true)][string]$Reason)
    $script:scanComplete = $false
    $script:stopScan = $true
    if (-not $script:limitReasons.Contains($Reason)) {
        $script:limitReasons.Add($Reason)
    }
}

function Add-ControlCandidate {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][string]$Surface,
        [Parameter(Mandatory = $true)][string]$Capability,
        [Parameter(Mandatory = $true)][string]$UpstreamFolderTrust
    )
    if ($script:stopScan) {
        return
    }
    $item = Get-Item -LiteralPath $Path -Force -ErrorAction Stop
    $key = $item.FullName.ToLowerInvariant()
    if ($script:seen.ContainsKey($key)) {
        return
    }
    $script:seen[$key] = $true
    if ($script:candidates.Count -ge $MaxCandidateFiles) {
        Stop-ControlScan -Reason 'candidate_file_limit_reached'
        return
    }
    $isReparse = (($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0)
    if ($item.PSIsContainer -or $isReparse) {
        $script:candidates.Add([ordered]@{
            relative_path = Get-RelativeControlPath -Root $projectRoot -FullPath $item.FullName
            path_kind = 'reparse_point'
            surface = $Surface
            capability = 'unknown'
            upstream_folder_trust = 'not_documented'
            bytes = $null
            sha256 = $null
        })
        return
    }
    $length = [long]$item.Length
    if (($script:totalBytes + $length) -gt $MaxCandidateBytes) {
        Stop-ControlScan -Reason 'candidate_byte_limit_reached'
        return
    }
    $script:totalBytes += $length
    $script:candidates.Add([ordered]@{
        relative_path = Get-RelativeControlPath -Root $projectRoot -FullPath $item.FullName
        path_kind = 'file'
        surface = $Surface
        capability = $Capability
        upstream_folder_trust = $UpstreamFolderTrust
        bytes = $length
        sha256 = (Get-FileHash -LiteralPath $item.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
    })
}

function Add-ControlDirectory {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][string]$Surface,
        [Parameter(Mandatory = $true)][string]$Capability,
        [Parameter(Mandatory = $true)][string]$UpstreamFolderTrust,
        [ValidateSet('all', 'markdown')][string]$Filter = 'all'
    )
    if ($script:stopScan -or -not (Test-Path -LiteralPath $Path)) {
        return
    }
    $rootItem = Get-Item -LiteralPath $Path -Force
    if (-not $rootItem.PSIsContainer) {
        Add-ControlCandidate -Path $rootItem.FullName -Surface $Surface -Capability $Capability -UpstreamFolderTrust $UpstreamFolderTrust
        return
    }
    if (($rootItem.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
        Add-ControlCandidate -Path $rootItem.FullName -Surface $Surface -Capability 'unknown' -UpstreamFolderTrust 'not_documented'
        return
    }
    $pending = New-Object System.Collections.Generic.Stack[string]
    $pending.Push($rootItem.FullName)
    while ($pending.Count -gt 0 -and -not $script:stopScan) {
        $current = $pending.Pop()
        foreach ($child in @(Get-ChildItem -LiteralPath $current -Force -ErrorAction Stop | Sort-Object FullName)) {
            if (($child.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
                Add-ControlCandidate -Path $child.FullName -Surface $Surface -Capability 'unknown' -UpstreamFolderTrust 'not_documented'
            } elseif ($child.PSIsContainer) {
                $pending.Push($child.FullName)
            } elseif ($Filter -eq 'all' -or $child.Extension -ieq '.md') {
                Add-ControlCandidate -Path $child.FullName -Surface $Surface -Capability $Capability -UpstreamFolderTrust $UpstreamFolderTrust
            }
            if ($script:stopScan) {
                break
            }
        }
    }
}

$instructionNames = @('Agents.md', 'Claude.md', 'CLAUDE.md', 'CLAUDE.local.md', 'AGENTS.md')
foreach ($scope in $scopeDirectories) {
    foreach ($name in $instructionNames) {
        $candidatePath = Join-Path $scope $name
        if (Test-Path -LiteralPath $candidatePath) {
            Add-ControlCandidate -Path $candidatePath -Surface 'project_instruction' -Capability 'instructions' -UpstreamFolderTrust 'not_documented'
        }
    }
    Add-ControlDirectory -Path (Join-Path $scope '.grok\rules') -Surface 'grok_rules' -Capability 'instructions' -UpstreamFolderTrust 'not_documented' -Filter markdown
    Add-ControlDirectory -Path (Join-Path $scope '.claude\rules') -Surface 'claude_rules' -Capability 'instructions' -UpstreamFolderTrust 'not_documented' -Filter markdown
    Add-ControlDirectory -Path (Join-Path $scope '.cursor\rules') -Surface 'cursor_rules' -Capability 'instructions' -UpstreamFolderTrust 'not_documented' -Filter markdown
    foreach ($entry in @(
        @{ Relative = '.grok\config.toml'; Surface = 'grok_project_config'; Capability = 'permissions_and_external_definitions'; Trust = 'partial' },
        @{ Relative = '.claude\settings.json'; Surface = 'claude_project_settings'; Capability = 'permissions_and_code_definitions'; Trust = 'partial' },
        @{ Relative = '.claude\settings.local.json'; Surface = 'claude_project_settings'; Capability = 'permissions_and_code_definitions'; Trust = 'partial' },
        @{ Relative = '.grok\lsp.json'; Surface = 'grok_lsp'; Capability = 'external_process'; Trust = 'required' },
        @{ Relative = '.cursor\hooks.json'; Surface = 'cursor_hooks'; Capability = 'code_execution'; Trust = 'required' },
        @{ Relative = '.cursor\mcp.json'; Surface = 'cursor_mcp'; Capability = 'external_process_or_connection'; Trust = 'required' }
    )) {
        $candidatePath = Join-Path $scope $entry.Relative
        if (Test-Path -LiteralPath $candidatePath) {
            Add-ControlCandidate -Path $candidatePath -Surface $entry.Surface -Capability $entry.Capability -UpstreamFolderTrust $entry.Trust
        }
    }
    Add-ControlDirectory -Path (Join-Path $scope '.grok\hooks') -Surface 'grok_hooks' -Capability 'code_execution' -UpstreamFolderTrust 'required'
    Add-ControlDirectory -Path (Join-Path $scope '.grok\plugins') -Surface 'grok_plugins' -Capability 'code_execution_or_external_connection' -UpstreamFolderTrust 'required'
    Add-ControlDirectory -Path (Join-Path $scope '.grok\skills') -Surface 'grok_skills' -Capability 'instructions_or_code' -UpstreamFolderTrust 'not_documented'
    Add-ControlDirectory -Path (Join-Path $scope '.grok\agents') -Surface 'grok_agents' -Capability 'instructions' -UpstreamFolderTrust 'not_documented'
    Add-ControlDirectory -Path (Join-Path $scope '.claude\skills') -Surface 'claude_skills' -Capability 'instructions_or_code' -UpstreamFolderTrust 'not_documented'
    Add-ControlDirectory -Path (Join-Path $scope '.cursor\skills') -Surface 'cursor_skills' -Capability 'instructions_or_code' -UpstreamFolderTrust 'not_documented'
}

$sortedCandidates = @($script:candidates | Sort-Object relative_path, surface)
$aggregateLines = @(
    $sortedCandidates | ForEach-Object {
        $bytesValue = if ($null -eq $_.bytes) { '-' } else { [string]$_.bytes }
        $shaValue = if ($null -eq $_.sha256) { '-' } else { $_.sha256 }
        "$($_.relative_path)|$($_.path_kind)|$($_.surface)|$($_.capability)|$($_.upstream_folder_trust)|$bytesValue|$shaValue"
    }
)
$aggregateText = if ($aggregateLines.Count -eq 0) { '' } else { ($aggregateLines -join "`n") + "`n" }
$aggregateSha256 = Get-TextSha256 -Text $aggregateText

if ($Decision -eq 'trusted') {
    if ($DecisionActor -eq 'sidecar-default' -or [string]::IsNullOrWhiteSpace($DecisionActor)) {
        throw 'A non-default DecisionActor is required for a trusted receipt.'
    }
    if ($ExpectedAggregateSha256 -notmatch '^[0-9a-fA-F]{64}$') {
        throw 'Trusted decision requires ExpectedAggregateSha256.'
    }
    if (-not $ExpectedAggregateSha256.Equals($aggregateSha256, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "Workspace control aggregate changed or was not approved. Observed: $aggregateSha256"
    }
    if (-not $script:scanComplete) {
        throw 'Trusted decision is forbidden when the control-surface scan is incomplete.'
    }
} elseif (-not [string]::IsNullOrWhiteSpace($ExpectedAggregateSha256)) {
    throw 'ExpectedAggregateSha256 is accepted only for a trusted decision.'
}

$launchPermitted = (
    $script:scanComplete -and (
        $Decision -eq 'trusted' -or
        ($Decision -eq 'restricted' -and $sortedCandidates.Count -eq 0)
    )
)
$limitations = New-Object System.Collections.Generic.List[string]
$limitations.Add('This receipt records filesystem control-surface candidates; it does not prove that Grok loaded, skipped, or safely interpreted them.')
$limitations.Add('Grok folder-trust separately gates documented project hook, MCP, LSP, and plugin code; this script does not modify that upstream trust store.')
$limitations.Add('Gitignore status is intentionally not applied, so the scan can over-report candidates that Grok may skip.')
if ($sortedCandidates.Count -gt 0 -and $Decision -eq 'restricted') {
    $limitations.Add('Restricted mode found project control surfaces and therefore does not permit a Grok launch from this workspace.')
}
foreach ($reason in $script:limitReasons) {
    $limitations.Add("Scan incomplete: $reason")
}

$receipt = [ordered]@{
    schema_version = '0.1.0'
    receipt_kind = 'grok-workspace-trust-preflight'
    receipt_id = 'TRUST-' + [guid]::NewGuid().ToString('N')
    created_at = (Get-Date).ToUniversalTime().ToString('o')
    workspace = [ordered]@{
        requested_path = $WorkspacePath
        canonical_path = $workspace
        project_root = $projectRoot
        scope_directories = @($scopeDirectories)
    }
    discovery = [ordered]@{
        scan_policy_version = 'grok-workspace-control-scan-v0.1'
        scan_policy_sha256 = $scanPolicySha256
        complete = $script:scanComplete
        max_candidate_files = $MaxCandidateFiles
        max_candidate_bytes = $MaxCandidateBytes
        candidate_count = $sortedCandidates.Count
        candidate_bytes = $script:totalBytes
        aggregate_sha256 = $aggregateSha256
        candidates = $sortedCandidates
        limit_reasons = @($script:limitReasons)
    }
    decision = [ordered]@{
        mode = $Decision
        actor = $DecisionActor
        explicit_digest_confirmation = ($Decision -eq 'trusted')
        launch_permitted = $launchPermitted
        expires_on_control_change = $true
    }
    upstream_folder_trust = [ordered]@{
        store_modified = $false
        documented_required_surfaces = @('project_hooks', 'project_mcp', 'project_lsp', 'project_plugin_code')
        sidecar_receipt_is_not_a_grok_trust_grant = $true
    }
    safety = [ordered]@{
        model_invoked = $false
        network_attempted = $false
        tools_executed = $false
        project_code_executed = $false
        file_contents_recorded = $false
        output_overwritten = $false
    }
    valid = $script:scanComplete
    limitations = @($limitations)
}

Write-Utf8Atomic -Path $output -Content (($receipt | ConvertTo-Json -Depth 12) + [Environment]::NewLine)
$receipt | ConvertTo-Json -Depth 12
if (-not $script:scanComplete) {
    exit 2
}
