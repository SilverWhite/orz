[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidateSet('add', 'remove', 'status')]
    [string]$Action,

    [Parameter(Mandatory = $true)]
    [ValidatePattern('^LIFGrokChild-[A-Za-z0-9-]+$')]
    [string]$RulePrefix,

    [string[]]$ProgramPath = @(),

    [string]$ProgramListJson
)

$ErrorActionPreference = 'Stop'
$identity = [System.Security.Principal.WindowsIdentity]::GetCurrent()
$principal = New-Object System.Security.Principal.WindowsPrincipal($identity)
if (-not $principal.IsInRole([System.Security.Principal.WindowsBuiltInRole]::Administrator)) {
    throw 'Administrator token required for temporary outbound firewall rules.'
}
foreach ($command in @('Get-NetFirewallProfile', 'Get-NetFirewallRule', 'New-NetFirewallRule', 'Remove-NetFirewallRule')) {
    if (-not (Get-Command $command -ErrorAction SilentlyContinue)) {
        throw "Required Windows Firewall command is unavailable: $command"
    }
}

$existing = @(Get-NetFirewallRule -Name "$RulePrefix-*" -ErrorAction SilentlyContinue)
if ($Action -eq 'status') {
    [ordered]@{
        schema_version = '0.1.0'
        action = 'status'
        rule_prefix = $RulePrefix
        remaining_rule_count = $existing.Count
        rule_names = @($existing | Sort-Object Name | ForEach-Object { $_.Name })
    } | ConvertTo-Json -Depth 10
    exit 0
}

if ($Action -eq 'remove') {
    $existing | Remove-NetFirewallRule -ErrorAction SilentlyContinue
    $remaining = @(Get-NetFirewallRule -Name "$RulePrefix-*" -ErrorAction SilentlyContinue)
    [ordered]@{
        schema_version = '0.1.0'
        action = 'remove'
        rule_prefix = $RulePrefix
        removed = $remaining.Count -eq 0
        remaining_rule_count = $remaining.Count
        rule_names = @($remaining | Sort-Object Name | ForEach-Object { $_.Name })
    } | ConvertTo-Json -Depth 10
    exit $(if ($remaining.Count -eq 0) { 0 } else { 2 })
}

if ($existing.Count -ne 0) {
    throw "Firewall rule prefix already exists: $RulePrefix"
}
$profiles = @(Get-NetFirewallProfile -ErrorAction Stop)
if ($profiles.Count -eq 0 -or @($profiles | Where-Object { -not $_.Enabled }).Count -gt 0) {
    throw 'All Windows Firewall profiles must be enabled before the child-tree probe.'
}
$requestedPrograms = if ($ProgramListJson) {
    @($ProgramListJson | ConvertFrom-Json)
} else {
    @($ProgramPath)
}
$programs = @(
    $requestedPrograms |
        ForEach-Object { (Resolve-Path -LiteralPath $_ -ErrorAction Stop).Path } |
        Sort-Object -Unique
)
if ($programs.Count -eq 0) {
    throw 'At least one exact program path is required.'
}

$created = New-Object System.Collections.Generic.List[string]
try {
    $index = 0
    foreach ($program in $programs) {
        $index += 1
        $specs = @(
            @{
                Suffix = "p$index-v4-nonloop"
                Remote = @(
                    '0.0.0.0/2', '64.0.0.0/3', '96.0.0.0/4', '112.0.0.0/5',
                    '120.0.0.0/6', '124.0.0.0/7', '126.0.0.0/8', '128.0.0.0/1'
                )
            },
            @{ Suffix = "p$index-v6-low"; Remote = '::/1' },
            @{ Suffix = "p$index-v6-high"; Remote = '8000::/1' }
        )
        foreach ($spec in $specs) {
            $name = "$RulePrefix-$($spec.Suffix)"
            New-NetFirewallRule `
                -Name $name `
                -DisplayName $name `
                -Direction Outbound `
                -Action Block `
                -Enabled True `
                -Profile Any `
                -Program $program `
                -Protocol Any `
                -RemoteAddress $spec.Remote | Out-Null
            $created.Add($name)
        }
    }
} catch {
    foreach ($name in $created) {
        Remove-NetFirewallRule -Name $name -ErrorAction SilentlyContinue
    }
    throw
}

[ordered]@{
    schema_version = '0.1.0'
    action = 'add'
    rule_prefix = $RulePrefix
    profiles_enabled = $true
    programs = $programs
    rule_count = $created.Count
    rule_names = @($created)
    nonloopback_ipv4_blocked = $true
    all_ipv6_blocked = $true
} | ConvertTo-Json -Depth 10
