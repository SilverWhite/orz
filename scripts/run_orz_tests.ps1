# run_orz_tests.ps1 — 0bc 杂项 FR2（2026-09-21）
# 目的：dogfood 会话把 ORZ_ACAF_FAIL_CLOSED=1 / ORZ_ACAF_MANIFEST /
# ORZ_ACAF_KEYSTORE / ORZ_ACAF_BINARY 沿进程树下沉给测试进程，使「起 run」
# 类测试在 fail-closed 下批量 panic 假红（案例 ORZ-ENV-POLLUTION-001）。
# 另两族同族实例（0bc 现场实证，一并清除）：
#   - ORZ_LIF_RLI_SHADOW=1（观测场姿态）会让 rli_reference_face_renders_
#     shadow_signal（首断言假定「未启用」）假红；
#   - GROK_HOME / GROK_AGENT（dogfood 载体值）会让 orz-host grok_home::
#     tests::* 四条（假定 GROK_HOME 未设）短路为 EnvRespected 假红。
# 本脚本在当前会话清除上述键（仅作用于本脚本启动的测试进程，不改生产
# fail-closed 纪律与功能开关语义），并给 RUST_MIN_STACK 一个默认值，然后
# 透传参数给 cargo。
# 0bd 升级（2026-09-22）：① 清除动作显式回显（清了哪些键）；② 修复
# PowerShell 5.1 吞裸 `--`（`& script ... -- --test-threads=1` 实测 `--`
# 不进 $args，测试参数被 cargo 当自有参数 ⇒ unexpected argument）——未见
# 分隔符时按测试二进制专属旗标启发式补回 `--`（脚本接口层，不代改模型命令）。
# 0bg 升级（2026-09-22）：④ **测试入口 auto 降并行档**——把 build_orz.ps1
# （0bd ①）的宿主提交余量自检引到测试入口（同阈值同口径；显式 -j/--jobs 照
# 用不改）；① **宿主套件红判据入口＝串行档**（追加 `-- --test-threads=1`）
# ——并行档的负载/计时敏感红（0bd §5）是已知边界，断言强度不放宽。
# 用法（从任意目录）：
#   powershell -File scripts\run_orz_tests.ps1 test -p orz-loop --lib -j 2
#   powershell -File scripts\run_orz_tests.ps1 test -p orz-loop --lib -- --test-threads=1
$orzRoot = Join-Path (Split-Path -Parent $PSScriptRoot) 'orz'
if (-not (Test-Path $orzRoot)) { Write-Error "orz workspace not found at $orzRoot"; exit 2 }
$cleared = @()
foreach ($k in @('ORZ_ACAF_FAIL_CLOSED','ORZ_ACAF_MANIFEST','ORZ_ACAF_KEYSTORE','ORZ_ACAF_BINARY','ORZ_LIF_RLI_SHADOW','GROK_HOME','GROK_AGENT')) {
    if (Test-Path "Env:$k") { Remove-Item "Env:$k" -ErrorAction SilentlyContinue; $cleared += $k }
}
if ($cleared.Count -gt 0) { Write-Host "[run_orz_tests] 已清会话污染 env（7 键防护）：$($cleared -join ', ')" }
if (-not $env:RUST_MIN_STACK) { $env:RUST_MIN_STACK = '67108864' }

# 0bd ③：裸 `--` 被 PowerShell 吞掉时，按测试二进制专属旗标（前缀匹配）补回
# 分隔符；`--` 已在参数里（-File 调用或带引号 `'--'`）则原样放行。
$passthroughFlags = @('--test-threads','--nocapture','--show-output','--include-ignored','--ignored','--exact','--skip','--shuffle','--report-time','--ensure-time')
$cargoArgs = @($args)
if ($cargoArgs -notcontains '--') {
    $idx = -1
    for ($i = 0; $i -lt $cargoArgs.Count; $i++) {
        $a = [string]$cargoArgs[$i]
        foreach ($flag in $passthroughFlags) {
            if ($a -eq $flag -or $a.StartsWith($flag + '=')) { $idx = $i; break }
        }
        if ($idx -ge 0) { break }
    }
    if ($idx -ge 0) {
        $head = if ($idx -gt 0) { @($cargoArgs[0..($idx - 1)]) } else { @() }
        $tail = @($cargoArgs[$idx..($cargoArgs.Count - 1)])
        $cargoArgs = @($head + @('--') + $tail)
        Write-Host '[run_orz_tests] 检测到被吞的前导 --：已在测试参数前补回分隔符（0bd ③）。'
    }
}
# 0bg ④：auto 降并行档（与 build_orz.ps1〔0bd ①〕同阈值同口径）——未显式
# 给 -j/--jobs 时按宿主提交余量选档：余量紧⇒1／偏紧⇒2／宽裕⇒min(核,8)。
$hasJobs = $false
foreach ($a in $cargoArgs) {
    $s = [string]$a
    if ($s -eq '-j' -or $s -eq '--jobs' -or $s -like '-j[0-9]*' -or $s -like '--jobs=*') { $hasJobs = $true; break }
}
if (-not $hasJobs -and $cargoArgs.Count -gt 0) {
    $os = Get-CimInstance Win32_OperatingSystem
    $commitTotalGB = $os.TotalVirtualMemorySize / 1MB
    $commitFreeGB  = $os.FreeVirtualMemory / 1MB
    $usedPct       = if ($commitTotalGB -gt 0) { [Math]::Round(100 * (1 - $commitFreeGB / $commitTotalGB), 1) } else { 0 }
    $cores         = [Environment]::ProcessorCount
    $jobsN = if ($usedPct -ge 88 -or $commitFreeGB -lt 4) { 1 }
             elseif ($usedPct -ge 72 -or $commitFreeGB -lt 12) { 2 }
             else { [Math]::Min($cores, 8) }
    $sep = -1
    for ($i = 0; $i -lt $cargoArgs.Count; $i++) { if ([string]$cargoArgs[$i] -eq '--') { $sep = $i; break } }
    $insertAt = if ($sep -ge 0) { $sep } else { $cargoArgs.Count }
    $head = if ($insertAt -gt 0) { @($cargoArgs[0..($insertAt - 1)]) } else { @() }
    $tail = if ($insertAt -lt $cargoArgs.Count) { @($cargoArgs[$insertAt..($cargoArgs.Count - 1)]) } else { @() }
    $cargoArgs = @($head + @('-j', "$jobsN") + $tail)
    Write-Host ("[run_orz_tests] auto 并行档 -j {0}（commit 使用 {1}% / 余 {2:n1} GiB；0bg ④；显式 -j 优先）" -f $jobsN, $usedPct, $commitFreeGB)
}
# 0bg ①：宿主套件红判据入口＝串行档——并行档无显式 test-threads 时给一行提示
# （0bd §5 定位：并行恒现 1 条＋浮动 1 条＝负载/计时敏感；断言强度不放宽）。
if (($cargoArgs -join ' ') -match 'orz-host' -and ($cargoArgs -join ' ') -notmatch 'test-threads') {
    Write-Host '[run_orz_tests] 提示（0bg ①）：orz-host 红判据入口＝串行档（追加 -- --test-threads=1）。'
}

Push-Location $orzRoot
try {
    & cargo @cargoArgs
    exit $LASTEXITCODE
} finally {
    Pop-Location
}