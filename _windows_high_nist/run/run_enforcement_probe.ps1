<#
    run_enforcement_probe.ps1 — Windows enforcement-probe runner（P0-0l-②）。

    用法：
      .\run_enforcement_probe.ps1 -Arm high-nist          # sandbox 模式（默认）
      .\run_enforcement_probe.ps1 -Arm control -Native    # 当前会话直跑
      .\run_enforcement_probe.ps1 -Arm non-admin -AllowlistIp 1.1.1.1

    -Sandbox 模式用 assurance/windows_sandbox.py 运行环境在对应墙内拉起探针：
      control   — 当前 token + Job containment
      non-admin — 受限 token（Administrators 禁用/deny-only、6 特权移除、
                  TokenVirtualizationAllowed=0）
      high-nist — 非 admin + LOW IL + AppContainer + Job + TEMP 重定向 +
                  egress wall
    探针退出码非 0 → 该臂不作数（fail-closed），runner 退出 1。
#>

[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidateSet('control', 'non-admin', 'high-nist')]
    [string]$Arm,

    [switch]$Native,

    [string]$Workspace = '',

    [string[]]$AllowlistIp = @(),

    [string]$ResultPath = '',

    [int]$TimeoutSeconds = 120
)

$ErrorActionPreference = 'Stop'
$root = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$probeScript = Join-Path $PSScriptRoot '..\policy\enforcement_probe.ps1'
$probeScript = (Resolve-Path $probeScript).Path

$tempWorkspace = $false
if (-not $Workspace) {
    $Workspace = Join-Path $env:TEMP "gsa-enforcement-$([guid]::NewGuid().ToString('N'))"
    New-Item -ItemType Directory -Path $Workspace -Force | Out-Null
    $tempWorkspace = $true
}
if (-not $ResultPath) {
    $ResultPath = Join-Path $Workspace "enforcement-probe-$Arm.json"
}

try {
    $reachIp = if ($AllowlistIp.Count -gt 0) { $AllowlistIp[0] } else { '' }
    if ($Native) {
        $nativeArgs = @(
            '-NoProfile', '-NonInteractive', '-ExecutionPolicy', 'Bypass',
            '-File', $probeScript,
            '-Arm', $Arm,
            '-Workspace', $Workspace,
            '-TempDir', (Join-Path $Workspace '.tmp'),
            '-ResultPath', $ResultPath
        )
        if ($reachIp) {
            $nativeArgs += @('-AllowlistReachabilityIp', $reachIp)
        }
        & powershell.exe @nativeArgs
        $probeExit = $LASTEXITCODE
    }
    else {
        $marker = Join-Path $Workspace '.assurance-p2-disposable.json'
        if (-not (Test-Path -LiteralPath $marker)) {
            @{
                schema_version = '0.1.0-draft'
                purpose = 'windows-native-sandbox-probe'
                allow_container_write_probe = $true
            } | ConvertTo-Json | Set-Content -LiteralPath $marker -Encoding ascii
        }
        $obsOutput = Join-Path $Workspace "windows-native-run-observation-$Arm.json"
        $args = @(
            (Join-Path $root 'scripts\run_windows_native_sandbox_command.py'),
            '--workspace', $Workspace,
            '--arm', $Arm,
            '--timeout', $TimeoutSeconds
        )
        foreach ($ip in $AllowlistIp) {
            $args += @('--allowlist-ip', $ip)
        }
        $args += @('--output', $obsOutput)
        # --command must be last: every following argument (including
        # child flags like -NoProfile) is passed verbatim to the child.
        $args += @(
            '--command',
            'powershell.exe', '-NoProfile', '-NonInteractive', '-ExecutionPolicy', 'Bypass',
            '-File', $probeScript,
            '-Arm', $Arm,
            '-Workspace', $Workspace,
            '-TempDir', (Join-Path $Workspace '.tmp'),
            '-ResultPath', $ResultPath
        )
        if ($reachIp) {
            $args += @('-AllowlistReachabilityIp', $reachIp)
        }

        $probeExit = 1
        python $args | Out-Host
        if ($LASTEXITCODE -ne 0) {
            Write-Error "run_windows_native_sandbox_command.py 失败（exit=$LASTEXITCODE）"
            exit 1
        }
        if (-not (Test-Path -LiteralPath $obsOutput)) {
            Write-Error "run observation 未生成：$obsOutput"
            exit 1
        }
        $obs = Get-Content -LiteralPath $obsOutput -Raw | ConvertFrom-Json
        $probeExit = [int]$obs.process.exit_code
        if ($obs.outcome -ne 'compliant') {
            Write-Error "run observation outcome=$($obs.outcome)（spawn 期墙未全过，见 diagnostics）— 该臂不作数（fail-closed）"
            exit 1
        }
    }

    if ($probeExit -ne 0) {
        Write-Error "enforcement-probe $Arm FAILED（exit=$probeExit）— 该臂不作数（fail-closed）"
        exit 1
    }
    Write-Host "enforcement-probe $Arm OK — result=$ResultPath"
    exit 0
}
finally {
    if ($tempWorkspace) {
        Remove-Item -LiteralPath $Workspace -Recurse -Force -ErrorAction SilentlyContinue
    }
}
