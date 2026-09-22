<#
.SYNOPSIS
  orz 构建入口（0bd ①）：构建前置自检＋自动降并行度（F14 宿主机 commit 上限缓解）。
.DESCRIPTION
  背景：本机 12 核 / 物理 16 GiB / 提交上限 ≈28.9 GiB；默认并行度的
  cargo/rustc（LLVM）在提交余量紧时会 OOM（0am FR2：exit 0xc0000409，
  `-j 1` 过；0bc/0be 惯例档 `-j 2`）。本脚本构建前读一次宿主机提交余量，
  据此选 `-j`（auto 档：余量紧 → 1；偏紧 → 2；宽裕 → min(核数, 8)），读数
  与选择打进清单；-Jobs 显式指定则照用。根因＝宿主 commit 上限，本脚本
  只是缓解、不根治（0bd ① 原措辞）。
.PARAMETER Release
  --release（载体重建/发布口径）。
.PARAMETER Package
  -p 目标（缺省 orz-bin）。
.PARAMETER Jobs
  并行度：auto（缺省，见上）或正整数（照用）。
.PARAMETER Check
  用 `cargo check` 代替 `cargo build`（快速核）。
.PARAMETER Clean
  先 `cargo clean`（大重建口径；慎用，显著拉长墙钟）。
.PARAMETER DryRun
  只做自检与清单打印，不调 cargo。
.EXAMPLE
  powershell -File scripts\build_orz.ps1 -Release -DryRun
.EXAMPLE
  powershell -File scripts\build_orz.ps1 -Release -Jobs 2
#>
param(
    [switch]$Release,
    [string]$Package = 'orz-bin',
    [string]$Jobs = 'auto',
    [switch]$Check,
    [switch]$Clean,
    [switch]$DryRun,
    [Parameter(ValueFromRemainingArguments = $true)][string[]]$Extra
)
$ErrorActionPreference = 'Stop'
$orzRoot = Join-Path (Split-Path -Parent $PSScriptRoot) 'orz'
if (-not (Test-Path $orzRoot)) { Write-Error "orz workspace not found at $orzRoot"; exit 2 }
# 前导 `--` 容错（0bd ③ 同族）：裸 `--` 被 PowerShell 吞掉/透传两种都收。
$extraArgs = @($Extra | Where-Object { $_ -ne '--' })

# ① 构建前置自检：宿主提交余量（commit 上限＝F14 根因面）
$os = Get-CimInstance Win32_OperatingSystem
$commitTotalGB = $os.TotalVirtualMemorySize / 1MB
$commitFreeGB  = $os.FreeVirtualMemory / 1MB
$memTotalGB    = $os.TotalVisibleMemorySize / 1MB
$memFreeGB     = $os.FreePhysicalMemory / 1MB
$usedPct       = if ($commitTotalGB -gt 0) { [Math]::Round(100 * (1 - $commitFreeGB / $commitTotalGB), 1) } else { 0 }
$cores         = [Environment]::ProcessorCount

# ② 并行度决策（auto 档：阈值＝本机经验先验，随读数复核；显式 -Jobs 照用）
if ($Jobs -eq 'auto') {
    $jobsN = if ($usedPct -ge 88 -or $commitFreeGB -lt 4) { 1 }
             elseif ($usedPct -ge 72 -or $commitFreeGB -lt 12) { 2 }
             else { [Math]::Min($cores, 8) }
    $jobsDisp = "$jobsN（auto：commit 使用 $usedPct% / 余 $('{0:n1}' -f $commitFreeGB) GiB）"
} else {
    $jobsN = [int]$Jobs
    $jobsDisp = "$jobsN（显式）"
}

Write-Host '== orz 构建装配清单（0bd ①） =='
Write-Host ("host       = {0} 核 / 物理 {1:n1} GiB（余 {2:n1}）/ commit {3:n1} GiB（余 {4:n1}，使用 {5}%）" -f `
    $cores, $memTotalGB, $memFreeGB, $commitTotalGB, $commitFreeGB, $usedPct)
Write-Host "jobs       = $jobsDisp"
$verb = if ($Check) { 'check' } else { 'build' }
$mode = if ($Release) { '--release' } else { '（debug）' }
Write-Host "cmd        = cargo $verb -p $Package $mode -j $jobsN $($extraArgs -join ' ')  [cwd=$orzRoot]"
if ($DryRun) { Write-Host '（-DryRun：只自检＋清单；未构建。）'; exit 0 }

Push-Location $orzRoot
try {
    if ($Clean) { & cargo clean; if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE } }
    $cargoCmd = @()
    if ($Check) { $cargoCmd += 'check' } else { $cargoCmd += 'build' }
    $cargoCmd += @('-p', $Package)
    if ($Release) { $cargoCmd += '--release' }
    $cargoCmd += @('-j', "$jobsN")
    $cargoCmd += $extraArgs
    & cargo @cargoCmd
    exit $LASTEXITCODE
} finally {
    Pop-Location
}
