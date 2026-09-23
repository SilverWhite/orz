<#
.SYNOPSIS
  ACP 外接客户端接入包装（2026-09-23）：装配 env 后 exec `orz --stdio`。
.DESCRIPTION
  orz 自身就是标准 ACP agent（stdio JSON-RPC）。宿主侧已实现：
    · 客户端请求 initialize / session/new / session/prompt
    · 客户端通知 session/cancel（回 PromptResponse stopReason=cancelled）
    · 出站 session/update 流式通知（客户端渲染对话流）
    · 出站 session/request_permission（客户端弹审批；非静默放行）
  外部客户端不能直接跑 `orz --stdio`，两个原因：
    ① ACAF 缺省 fail-closed——未配 ORZ_ACAF_MANIFEST／ORZ_ACAF_KEYSTORE
       时运行被拒（D-15），不是降级放行；
    ② 没带 `--real` 会落到 FakeProvider（脚本化假模型）。
  故统一走本包装。**stdout 是 ACP 的 JSON-RPC 通道，正常运行期一律不写
  任何东西到 stdout／host**；诊断只走 stderr，装配清单只在 -DryRun 打印。
  模型凭据不需要在此注入：Windows 走凭据管理器（ADR-0006），
  仅当需要覆盖时才设 ORZ_DEEPSEEK_API_KEY。
.PARAMETER Client
  zed | vscode | jetbrains | custom —— 只影响 -DryRun 打印的配置片段。
.PARAMETER ReadOnly
  不带 --allow-write／--allow-shell／--allow-network（只读姿态）。
.PARAMETER FakeProvider
  自检用：不带 `--real`，落到脚本化 FakeProvider（零模型调用）。仅用于验证
  接入链路本身（PowerShell → orz 的 stdio 透传），正式使用不要带。
.EXAMPLE
  powershell -File scripts/orz_acp_launch.ps1 -Client zed -DryRun
.EXAMPLE
  # 客户端里注册的 command/args（Zed settings.json → agent_servers）
  powershell -NoProfile -File D:\CLI\scripts\orz_acp_launch.ps1
#>
param(
    [string]$Workspace = (Get-Location).Path,
    [string]$BinDir = 'D:\tb-eval\orz-windows',
    [string]$AcafRoot = 'D:\tb-eval\orz-windows\acaf',
    [string]$MaxWallclock = '0',
    [ValidateSet('zed', 'vscode', 'jetbrains', 'custom')][string]$Client = 'zed',
    [switch]$RliOff,
    [switch]$ReadOnly,
    [switch]$FakeProvider,
    [switch]$DryRun
)
$ErrorActionPreference = 'Stop'

function Assert-True($cond, $msg) { if (-not $cond) { throw "断言失败：$msg" } }
function Say([string]$m) { if ($DryRun) { Write-Host $m } else { [Console]::Error.WriteLine($m) } }

# ① 工作目录断言（沿 dogfood_launch：不得落在 orz 子模块里）
$ws = (Resolve-Path -LiteralPath $Workspace).Path
Assert-True (Test-Path -LiteralPath $ws -PathType Container) "工作目录不存在：$ws"
Assert-True ($ws -notmatch '\\orz(\\|$)') "工作目录不得为 orz 子模块（实得 $ws）"

# ② 载体三件套
$orz    = Join-Path $BinDir 'orz.exe'
$signer = Join-Path $BinDir 'orz-signer.exe'
$provision = Join-Path $BinDir 'orz-acaf-provision.exe'
foreach ($f in @($orz, $signer, $provision)) { Assert-True (Test-Path -LiteralPath $f) "载体缺件：$f" }
$ver = (Get-Item $orz).VersionInfo.ProductVersion
if ([string]::IsNullOrWhiteSpace($ver)) { $ver = 'unknown' }
# 用 .NET 算哈希而不是 `Get-FileHash`：被编辑器/客户端这类外部进程拉起时，
# `Microsoft.PowerShell.Utility` 未必随 PSModulePath 自动装载（实测被 Python
# 子进程拉起即 CommandNotFoundException）。此路径只为装配清单可读性，失败不阻断。
$sha = try {
    $h = [System.Security.Cryptography.SHA256]::Create()
    try { ([System.BitConverter]::ToString($h.ComputeHash([System.IO.File]::ReadAllBytes($orz)))).Replace('-', '') }
    finally { $h.Dispose() }
} catch { '' }

# ③ ACAF：manifest 在册则复用，否则现场 provision（与狗粮启动器同形）
$manifest = Join-Path $AcafRoot 'signer-manifest.json'
$keystore = Join-Path $AcafRoot 'keystore'
if (-not (Test-Path -LiteralPath $manifest)) {
    if ($DryRun) {
        Say "[acp] ACAF manifest 缺失（DryRun 不 provision）：$manifest"
    } else {
        New-Item -ItemType Directory -Path $AcafRoot -Force | Out-Null
        & $provision $AcafRoot $manifest
        Assert-True ($LASTEXITCODE -eq 0) "ACAF provisioning 失败（exit $LASTEXITCODE）"
    }
}

# ④ env 装配
$env:ORZ_ACAF_MANIFEST    = $manifest
$env:ORZ_ACAF_KEYSTORE    = $keystore
$env:ORZ_ACAF_BINARY      = $signer
$env:ORZ_ACAF_FAIL_CLOSED = '1'
$env:ORZ_LIF_RLI_SHADOW   = if ($RliOff) { '0' } else { '1' }
if ($MaxWallclock -ne '0') { $env:ORZ_MAX_WALLCLOCK = $MaxWallclock }
$protoc = 'D:\tb-eval\.tools\protoc-25.3\bin\protoc.exe'
if (Test-Path -LiteralPath $protoc) { $env:PROTOC = $protoc }
$grokHome = Join-Path $BinDir 'grok-home'
if (Test-Path -LiteralPath $grokHome) { $env:GROK_HOME = $grokHome; $env:GROK_AGENT = '1' }

$flags = @('--stdio')
if (-not $FakeProvider) { $flags += '--real' }
if (-not $ReadOnly) { $flags += @('--allow-write', '--allow-shell', '--allow-network') }

if ($DryRun) {
    Write-Host '== ACP 外接接入装配清单 =='
    Write-Host "workspace  = $ws（断言通过：非 orz 子模块）"
    Write-Host "carrier    = $orz（v$ver；sha256 $($sha.Substring(0,12))…）"
    Write-Host "acaf       = manifest=$manifest / keystore=$keystore / signer=$signer（fail-closed=1）"
    Write-Host "rli        = $(if ($RliOff) { 'off（-RliOff）' } else { 'on（缺省常开）' })（ORZ_LIF_RLI_SHADOW=$($env:ORZ_LIF_RLI_SHADOW)）"
    Write-Host "flags      = $($flags -join ' ')"
    Write-Host "credential = Windows 凭据管理器（ADR-0006；未设 ORZ_DEEPSEEK_API_KEY 时走此路）"
    Write-Host '=== stdout/stdin 语义 ==='
    Write-Host 'stdin/stdout = ACP JSON-RPC 通道；正常运行期本包装不写 stdout（诊断只走 stderr）'
    Write-Host '=== 客户端配置片段 ==='
    $snippet = @'
{ "agent_servers": { "Orz": { "type": "custom", "command": "powershell", "args": ["-NoProfile","-ExecutionPolicy","Bypass","-File","D:\CLI\scripts\orz_acp_launch.ps1"] } } }
'@
    switch ($Client) {
        'zed'       { Write-Host '（Zed settings.json → agent_servers，ACP 自定义代理；cwd 即工作区根）'; Write-Host $snippet }
        'vscode'    { Write-Host '（VS Code：Exo 等 ACP 客户端，command/args 同上）'; Write-Host $snippet }
        'jetbrains' { Write-Host '（JetBrains AI Assistant → ACP，command/args 同上）'; Write-Host $snippet }
        default     { Write-Host '（任取 ACP 客户端：command=powershell，args=本脚本）'; Write-Host $snippet }
    }
    Write-Host '（-DryRun：装配与断言完成；未启动。）'
    exit 0
}

# ⑤ 启动：stdout 交给 ACP，包装本身保持沉默
Push-Location -LiteralPath $ws
try {
    & $orz @flags
    $code = $LASTEXITCODE
} finally {
    Pop-Location
}
exit $code
