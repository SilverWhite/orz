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
    # 0bh ⑥（2026-09-22）：余量排队口径（显式开关；缺省关＝行为不变）。
    [switch]$WaitForHeadroom,
    [Parameter(ValueFromRemainingArguments = $true)][string[]]$Extra
)
$ErrorActionPreference = 'Stop'
$orzRoot = Join-Path (Split-Path -Parent $PSScriptRoot) 'orz'
if (-not (Test-Path $orzRoot)) { Write-Error "orz workspace not found at $orzRoot"; exit 2 }
# 前导 `--` 容错（0bd ③ 同族）：裸 `--` 被 PowerShell 吞掉/透传两种都收。
$extraArgs = @($Extra | Where-Object { $_ -ne '--' })

# 0.6.10 重建轮补（0bd ① 同族）：构建前置补齐 `PROTOC`——`orz-tools-api`
# 的 build.rs 走 tonic_build，宿主缺 `protoc` 时在**依赖阶段**即失败（exit 101），
# 此前只有狗粮启动器装配该键（scripts/dogfood_launch.ps1 同路径）。存在即设入，
# 仅回显；不存在时下面清单行会显式报『未设』，不静默。
$protoc = 'D:\tb-eval\.tools\protoc-25.3\bin\protoc.exe'
if (-not $env:PROTOC -and (Test-Path -LiteralPath $protoc)) { $env:PROTOC = $protoc }

# ① 构建前置自检：宿主提交余量（commit 上限＝F14 根因面）
$os = Get-CimInstance Win32_OperatingSystem
$commitTotalGB = $os.TotalVirtualMemorySize / 1MB
$commitFreeGB  = $os.FreeVirtualMemory / 1MB
$memTotalGB    = $os.TotalVisibleMemorySize / 1MB
$memFreeGB     = $os.FreePhysicalMemory / 1MB
$usedPct       = if ($commitTotalGB -gt 0) { [Math]::Round(100 * (1 - $commitFreeGB / $commitTotalGB), 1) } else { 0 }
$cores         = [Environment]::ProcessorCount

# ② 并行度决策（auto 档：阈值＝本机经验先验，随读数复核；显式 -Jobs 照用）
# 0bh ⑥（2026-09-22）：争用面收口——`-WaitForHeadroom` 排队口径（显式开关，
# 缺省关＝行为不变）；启动前额外做一次**同名重活进程扫描**（cargo/rustc/orz）
# 与软提示；余量不足且开关开启时按 15 s 节拍轮询等待（≤30 min，超时如实报出）。
if ($WaitForHeadroom -and $Jobs -eq 'auto') {
    $waitDeadline = (Get-Date).AddMinutes(30)
    while ($true) {
        $osProbe = Get-CimInstance Win32_OperatingSystem
        $freeProbe = $osProbe.FreeVirtualMemory / 1MB
        $usedProbe = if ($osProbe.TotalVirtualMemorySize -gt 0) { 100 * (1 - $osProbe.FreeVirtualMemory / $osProbe.TotalVirtualMemorySize) } else { 0 }
        $heavy = @(Get-Process -Name cargo, rustc, orz -ErrorAction SilentlyContinue).Count
        if (($usedProbe -lt 72 -and $freeProbe -ge 12) -or (Get-Date) -gt $waitDeadline) {
            Write-Host ("争用面     = 重活进程 {0} 个；commit 使用 {1:n1}% / 余 {2:n1} GiB（{3}）" -f `
                $heavy, $usedProbe, $freeProbe, $(if ((Get-Date) -gt $waitDeadline) { '等待超时，按当前余量继续' } else { '已达开跑线' }))
            break
        }
        Write-Host ("[等待余量] 争用中：重活进程 {0} 个；commit 使用 {1:n1}% / 余 {2:n1} GiB（15 s 后重试；-WaitForHeadroom）" -f `
            $heavy, $usedProbe, $freeProbe)
        Start-Sleep -Seconds 15
    }
}
if ($Jobs -eq 'auto') {
    $jobsN = if ($usedPct -ge 88 -or $commitFreeGB -lt 4) { 1 }
             elseif ($usedPct -ge 72 -or $commitFreeGB -lt 12) { 2 }
             else { [Math]::Min($cores, 8) }
    $jobsDisp = "$jobsN（auto：commit 使用 $usedPct% / 余 $('{0:n1}' -f $commitFreeGB) GiB）"
    # 0bh ⑥ 软提示（不阻断）：紧余量/高使用率时如实提示争用面与降并依据。
    if ($usedPct -ge 72 -or $commitFreeGB -lt 12) {
        Write-Host ("软提示     = commit 余量偏紧（使用 {0}%、余 {1:n1} GiB）——已按 auto 档降并行；" -f $usedPct, $commitFreeGB) -NoNewline
        Write-Host "如与其它重活并跑，可用 -WaitForHeadroom 排队或稍后重试（判定归你，脚本不代办）。"
    }
} else {
    $jobsN = [int]$Jobs
    $jobsDisp = "$jobsN（显式）"
}

Write-Host '== orz 构建装配清单（0bd ①） =='
Write-Host ("host       = {0} 核 / 物理 {1:n1} GiB（余 {2:n1}）/ commit {3:n1} GiB（余 {4:n1}，使用 {5}%）" -f `
    $cores, $memTotalGB, $memFreeGB, $commitTotalGB, $commitFreeGB, $usedPct)
Write-Host "jobs       = $jobsDisp"
Write-Host "protoc     = $(if ($env:PROTOC) { $env:PROTOC } else { '（未设——依赖 protoc 的目标会失败）' })"
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
