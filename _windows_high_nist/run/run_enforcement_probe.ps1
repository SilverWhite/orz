<#
    run_enforcement_probe.ps1 — Windows enforcement-probe runner（P0-0l-②）。

    用法：
      .\run_enforcement_probe.ps1 -Arm high-nist          # sandbox 模式（默认）
      .\run_enforcement_probe.ps1 -Arm control -Native    # 当前会话直跑
      .\run_enforcement_probe.ps1 -Arm non-admin -AllowlistIp 1.1.1.1
      .\run_enforcement_probe.ps1 -Arm high-nist -AllowlistIp 1.1.1.1 `
          -NoAppcontainer   # 生产墙（no-AC + allowlist，TER T2.2 同接线）

    -Sandbox 模式用 assurance/windows_sandbox.py 运行环境在对应墙内拉起探针：
      control   — 当前 token + Job containment
      non-admin — 受限 token（Administrators 禁用/deny-only、6 特权移除、
                  TokenVirtualizationAllowed=0）
      high-nist — 非 admin + LOW IL + AppContainer + Job + TEMP 重定向 +
                  egress wall
      high-nist -NoAppcontainer — 同上但不含 AppContainer 层（2026-09-03
                  裁决：orz.exe 在 AppContainer 下 DLL init 失败；agent
                  runner 的 allowlist 墙即此形态）。子探针跳过
                  appcontainer_token 断言（mode=disabled）。
    探针退出码非 0 → 该臂不作数（fail-closed），runner 退出 1。
#>

[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidateSet('control', 'non-admin', 'high-nist')]
    [string]$Arm,

    [switch]$Native,

    [switch]$NoAppcontainer,

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
    if ($NoAppcontainer) {
        if ($Arm -ne 'high-nist') {
            Write-Error "-NoAppcontainer 只对 -Arm high-nist 有意义"
            exit 1
        }
        if ($Native) {
            Write-Error "-NoAppcontainer 不能与 -Native 同用（Native 不经 sandbox）"
            exit 1
        }
    }
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
        $pyProbeScript = Join-Path $PSScriptRoot '..\policy\enforcement_probe.py'
        $pyProbeScript = (Resolve-Path $pyProbeScript).Path
        # Copy the probe into the workspace only for AppContainer-enabled
        # spawns: AppContainer children can only read paths granted to the
        # package SID / ALL APPLICATION PACKAGES (C:\s4 is not), and the
        # sandbox grants the workspace to the AppContainer SID at spawn time.
        # With -NoAppcontainer the child is the plain (restricted) run user,
        # which reads C:\s4 normally (same layout as orz.exe in agent runs),
        # so the probe runs in place.
        if (-not $NoAppcontainer) {
            $wsProbe = Join-Path $Workspace 'enforcement_probe.py'
            Copy-Item -LiteralPath $pyProbeScript -Destination $wsProbe -Force
            $pyProbeScript = $wsProbe
        }
        $args = @(
            (Join-Path $root 'scripts\run_windows_native_sandbox_command.py'),
            '--workspace', $Workspace,
            '--arm', $Arm,
            '--timeout', $TimeoutSeconds
        )
        if ($NoAppcontainer) {
            $args += '--no-appcontainer'
        }
        foreach ($ip in $AllowlistIp) {
            $args += @('--allowlist-ip', $ip)
        }
        $args += @('--output', $obsOutput)
        # --command must be last: every following argument (including
        # child flags like -NoProfile) is passed verbatim to the child.
        $args += @(
            '--command',
            # Python probe: the sandboxed child must be AppContainer-capable.
            # powershell.exe (Windows PowerShell 5.1 / .NET Framework CLR)
            # fails DLL initialization as an AppContainer process on
            # Windows 11 25H2 (STATUS_DLL_INIT_FAILED / 0xC0000142).
            'python.exe', $pyProbeScript,
            '-Arm', $Arm,
            '-Workspace', $Workspace,
            '-TempDir', (Join-Path $Workspace '.tmp'),
            '-ResultPath', $ResultPath
        )
        if ($reachIp) {
            $args += @('-AllowlistReachabilityIp', $reachIp)
        }
        if ($NoAppcontainer) {
            $args += @('-ExpectAppcontainer', '0')
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
        $probeExit = [int64]$obs.process.exit_code
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
