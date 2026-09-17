<#
.SYNOPSIS
  狗粮 run 启动器（FR-A14 处置，2026-09-17）：env＋信任＋载体路径＋显式 cwd 断言一次装配。
.DESCRIPTION
  0am M-1 误启动（cwd 残留 orz 子模块、4.5 分钟活跃 WIP 代价；主因归运行方）
  与 0ai 收尾建议两次坐实后固化。启动前断言：
    ① cwd 必须为工作区根、且不得为 orz 子模块；
    ② 载体三件套（orz.exe / orz-signer.exe / orz-acaf-provision.exe）在册；
    ③ ACAF manifest 复用或现场 provision；
    ④ 题面文件可读。
  全部通过才起跑；-DryRun 只装配与打印，不启动。
.PARAMETER TaskFile
  题面文件路径（相对 $Workspace 或绝对路径；惯例 .tmp-*-task.txt）。
.EXAMPLE
  powershell -File scripts/dogfood_launch.ps1 -TaskFile .tmp-friction-task.txt -DryRun
.EXAMPLE
  powershell -File scripts/dogfood_launch.ps1 -TaskFile .tmp-0am-task.txt
#>
param(
    [string]$Workspace = 'D:\CLI',
    [string]$BinDir = 'D:\tb-eval\orz-windows',
    [string]$AcafRoot = 'D:\tb-eval\orz-windows\acaf',
    [Parameter(Mandatory = $true)][string]$TaskFile,
    [string]$MaxWallclock = '0',
    [switch]$Shadow,
    [switch]$DryRun
)
$ErrorActionPreference = 'Stop'

function Assert-True($cond, $msg) { if (-not $cond) { throw "断言失败：$msg" } }

# ① cwd 断言（M-1 预防；启动前 pwd 入 checklist）
$ws = (Resolve-Path -LiteralPath $Workspace).Path
Assert-True ($ws -notmatch '\\orz(\\|$)') "workspace 不得为 orz 子模块（M-1 误启动形态；实得 $ws）"
Assert-True ($ws -eq 'D:\CLI') "workspace 必须为 D:\CLI（实得 $ws）"

$taskPath = if ([System.IO.Path]::IsPathRooted($TaskFile)) { $TaskFile } else { Join-Path $ws $TaskFile }
Assert-True (Test-Path -LiteralPath $taskPath) "题面文件不存在：$taskPath"
$taskPath = (Resolve-Path -LiteralPath $taskPath).Path
$prompt = Get-Content -LiteralPath $taskPath -Raw
Assert-True ($prompt.Trim().Length -gt 0) "题面文件为空：$taskPath"

# ② 载体三件套
$orz       = Join-Path $BinDir 'orz.exe'
$signer    = Join-Path $BinDir 'orz-signer.exe'
$provision = Join-Path $BinDir 'orz-acaf-provision.exe'
foreach ($f in @($orz, $signer, $provision)) { Assert-True (Test-Path -LiteralPath $f) "载体缺件：$f" }
# Rust 载体不带 VersionInfo ⇒ ProductVersion 常为空串（DryRun 显示「v；」）；
# 空时回落 bump 记录口径的「unknown」（主会话复核补，2026-09-18）。
$ver = (Get-Item $orz).VersionInfo.ProductVersion
if ([string]::IsNullOrWhiteSpace($ver)) { $ver = 'unknown' }
$sha = (Get-FileHash -LiteralPath $orz -Algorithm SHA256).Hash

# ③ ACAF：manifest 在册则复用，否则现场 provision（与 orz_acaf_run.ps1 同形）
$manifest = Join-Path $AcafRoot 'signer-manifest.json'
$keystore = Join-Path $AcafRoot 'keystore'
if (-not (Test-Path -LiteralPath $manifest)) {
    New-Item -ItemType Directory -Path $AcafRoot -Force | Out-Null
    & $provision $AcafRoot $manifest
    Assert-True ($LASTEXITCODE -eq 0) "ACAF provisioning 失败（exit $LASTEXITCODE）"
}

# ④ env 装配（0am 口径：无墙钟=0；权限三键；PROTOC；grok home）
$env:ORZ_MAX_WALLCLOCK    = $MaxWallclock
$env:ORZ_REAL             = '1'
$env:ORZ_ALLOW_WRITE      = '1'
$env:ORZ_ALLOW_SHELL      = '1'
$env:ORZ_ALLOW_NETWORK    = '1'
$env:ORZ_ACAF_MANIFEST    = $manifest
$env:ORZ_ACAF_KEYSTORE    = $keystore
$env:ORZ_ACAF_BINARY      = $signer
$env:ORZ_ACAF_FAIL_CLOSED = if ($Shadow) { '0' } else { '1' }
$protoc = 'D:\tb-eval\.tools\protoc-25.3\bin\protoc.exe'
if (Test-Path -LiteralPath $protoc) { $env:PROTOC = $protoc }
$grokHome = Join-Path $BinDir 'grok-home'
if (Test-Path -LiteralPath $grokHome) { $env:GROK_HOME = $grokHome; $env:GROK_AGENT = '1' }

# ⑤ 装配清单（启动前 checklist）
Write-Host '== 狗粮启动装配清单 =='
Write-Host "cwd        = $ws（断言通过：非 orz 子模块）"
$verDisp = if ($ver -eq 'unknown') { 'unknown' } else { "v$ver" }
Write-Host "carrier    = $orz（$verDisp；sha256 $($sha.Substring(0,12))…）"
Write-Host "task       = $taskPath（$($prompt.Length) 字符）"
Write-Host "env        = MAX_WALLCLOCK=$MaxWallclock / ACAF fail-closed=$(-not $Shadow) / PROTOC=$($env:PROTOC) / GROK_HOME=$($env:GROK_HOME)"
Write-Host 'flags      = --real --allow-write --allow-shell --allow-network -p <task>'
if ($DryRun) { Write-Host '（-DryRun：装配与断言全部通过；未启动。）'; exit 0 }

# ⑥ 启动（日志落 .tmp-*；门禁忽略前缀）
$stamp = Get-Date -Format 'yyyyMMdd-HHmmss'
$log = Join-Path $ws ".tmp-dogfood-$stamp.log"
Write-Host "launching（日志：$log）"
Push-Location -LiteralPath $ws
try {
    & $orz -p $prompt --real --allow-write --allow-shell --allow-network *>&1 | Tee-Object -FilePath $log
    exit $LASTEXITCODE
} finally { Pop-Location }
