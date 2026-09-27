<#
.SYNOPSIS
  狗粮 run 启动器（FR-A14 处置，2026-09-17）：env＋信任＋载体路径＋显式 cwd 断言一次装配。
.DESCRIPTION
  0am M-1 误启动（cwd 残留 orz 子模块、4.5 分钟活跃 WIP 代价；主因归运行方）
  与 0ai 收尾建议两次坐实后固化。启动前断言：
    ① cwd 必须为工作区根、且不得为 orz 子模块；
    ② 载体三件套（orz.exe / orz-signer.exe / orz-acaf-provision.exe）在册；
    ③ ACAF manifest 复用或现场 provision，且**密钥库根＝provision 落点**
    （0by S2：此处曾多拼一层 `\keystore`，新 bin 目录下签名器启动即退、
    票据类工具面全灭）；
    ④ 题面文件可读。
  全部通过才起跑；-DryRun 只装配与打印，不启动。
  0bd 增补（2026-09-22）：⑤⑧ RLI 开关显式入装配清单（缺省 on＝常开；-RliOff
  置 kill switch）；⑭ 题面按显式 UTF-8 读取（PS 5.1 缺省按 ANSI 读无 BOM
  题面 ⇒ 整篇乱码，0be 轮实测）。
  0bs ②（2026-09-25）：⑮ 控制台输出编码显式钉 UTF-8（[Console]::OutputEncoding
  ＋$OutputEncoding）——Tee/捕获管道下原生输出按 ANSI 解码呈 GBK 乱码
  （0bm F6 一族；修复后日志中文与机械文案可读）。
  0bt②（2026-09-26）：⑯ 版本旁路——carrier 版本读载体自带 `--build-info`
  （打包面单一源；Rust 载体无 Win32 VersionInfo，不再显示 unknown）。
.PARAMETER TaskFile
  题面文件路径（相对 $Workspace 或绝对路径；惯例 .tmp-*-task.txt）。
.PARAMETER RliOff
  关闭 RLI（kill switch：写 ORZ_LIF_RLI_SHADOW=0）。缺省显式写 1＝常开
  （0bf ① 起语义反转：未设/1 = on；0/off/false/no = off）。
.EXAMPLE
  powershell -File scripts/dogfood_launch.ps1 -TaskFile .tmp-friction-task.txt -DryRun
.EXAMPLE
  powershell -File scripts/dogfood_launch.ps1 -TaskFile .tmp-0am-task.txt
.EXAMPLE
  powershell -File scripts/dogfood_launch.ps1 -TaskFile .tmp-0bd-task.txt -RliOff -DryRun
#>
param(
    [string]$Workspace = 'D:\CLI',
    [string]$BinDir = 'D:\tb-eval\orz-windows',
    [string]$AcafRoot = 'D:\tb-eval\orz-windows\acaf',
    [Parameter(Mandatory = $true)][string]$TaskFile,
    [string]$MaxWallclock = '0',
    [switch]$Shadow,
    [switch]$RliOff,
    [switch]$DryRun
)
$ErrorActionPreference = 'Stop'

# ②（0bs，2026-09-25）：**控制台输出编码链显式钉 UTF-8**——`Tee-Object`／
# 重定向捕获下，PS 5.1 按 ANSI（本机 GB2312）解码原生应用输出，中文与机械面
# 文案呈 GBK 乱码（0bm F6 一族、本会话 git diff 同族实读）。消费者侧口径：
# `[Console]::OutputEncoding`＝读原生输出所用编码；`$OutputEncoding`＝管道
# 发给原生命令所用的编码。载体侧 orz 已自钉 SetConsoleOutputCP(65001)，
# 本处补的是**捕获链**（launcher 是固定的消费入口）。
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
$OutputEncoding = [System.Text.Encoding]::UTF8

function Assert-True($cond, $msg) { if (-not $cond) { throw "断言失败：$msg" } }

# ②（0bs）自检钉：控制台输出编码必须是 UTF-8（见文件头 ⑮ 注；失败即抛）。
Assert-True ([Console]::OutputEncoding.WebName -eq 'utf-8') "控制台输出编码须为 UTF-8（实得 $([Console]::OutputEncoding.WebName)）"

# ① cwd 断言（M-1 预防；启动前 pwd 入 checklist）
$ws = (Resolve-Path -LiteralPath $Workspace).Path
Assert-True ($ws -notmatch '\\orz(\\|$)') "workspace 不得为 orz 子模块（M-1 误启动形态；实得 $ws）"
Assert-True ($ws -eq 'D:\CLI') "workspace 必须为 D:\CLI（实得 $ws）"

$taskPath = if ([System.IO.Path]::IsPathRooted($TaskFile)) { $TaskFile } else { Join-Path $ws $TaskFile }
Assert-True (Test-Path -LiteralPath $taskPath) "题面文件不存在：$taskPath"
$taskPath = (Resolve-Path -LiteralPath $taskPath).Path
# ⑭（0bd）：读取端显式 UTF-8——PS 5.1 缺省按 ANSI（本机 GB2312）读**无 BOM**
# 的 UTF-8 题面会整篇乱码（0be 轮 journal run_started.payload.prompt 实测
# `璇峰厛鏌ョ湅…`）；显式 UTF-8 对带/不带 BOM 两种 UTF-8 都正确。
$prompt = Get-Content -LiteralPath $taskPath -Raw -Encoding UTF8
Assert-True ($prompt.Trim().Length -gt 0) "题面文件为空：$taskPath"
$taskBytes = [System.IO.File]::ReadAllBytes($taskPath)
$taskBom = ($taskBytes.Length -ge 3 -and $taskBytes[0] -eq 0xEF -and `
    $taskBytes[1] -eq 0xBB -and $taskBytes[2] -eq 0xBF)
$taskEncDisp = if ($taskBom) { 'utf-8 BOM' } else { 'utf-8 无 BOM' }
# 0bg ⑥（2026-09-22，写入端强制 BOM）：题面磁盘形态统一为 UTF-8 **带 BOM**。
# 读取端（0bd ⑭）已显式 UTF-8，但链上/链下仍有按 ANSI 读题面者（0be 轮
# 实测乱码）；启动器是唯一机械点——缺 BOM ⇒ 原地以带 BOM 形态重写（内容经
# 显式 UTF-8 读回，字节语义不变）；读回含替换字符（U+FFFD＝文件真身非
# UTF-8／损坏）⇒ 显式断言失败，不给「侥幸猜对」留门。
if ($prompt.Contains([char]0xFFFD)) {
    Assert-True $false "题面不是有效 UTF-8（读回含替换字符 U+FFFD）：$taskPath —— 请以 UTF-8 保存题面（0bd ⑭／0bg ⑥）"
}
if (-not $taskBom) {
    [System.IO.File]::WriteAllText($taskPath, $prompt, (New-Object System.Text.UTF8Encoding($true)))
    $taskBom = $true
    $taskEncDisp = 'utf-8 BOM（启动器写入端已补）'
    Write-Host "[dogfood_launch] 题面缺 BOM ⇒ 已原地重写为 UTF-8 带 BOM（0bg ⑥ 写入端强制）：$taskPath"
}

# ② 载体三件套
$orz       = Join-Path $BinDir 'orz.exe'
$signer    = Join-Path $BinDir 'orz-signer.exe'
$provision = Join-Path $BinDir 'orz-acaf-provision.exe'
foreach ($f in @($orz, $signer, $provision)) { Assert-True (Test-Path -LiteralPath $f) "载体缺件：$f" }
# 0bt②（2026-09-26）：版本改读**载体自带旁路** `--build-info`——Rust 载体不带
# Win32 VersionInfo（ProductVersion 常为空串，DryRun 曾显示 carrier=unknown；
# 2026-09-18 主会话复核补）。载体自报＝打包面版本（单一源＝CARGO_PKG_VERSION）；
# 老载体不支持该旗标时回落「unknown」（如实回落，不伪造 bump 记录口径）。
$ver = 'unknown'
try {
    $buildInfo = (& $orz --build-info 2>$null | Select-Object -First 1)
    if ($LASTEXITCODE -eq 0 -and $buildInfo -match 'version=([^\s]+)') { $ver = $Matches[1] }
} catch {
    $ver = 'unknown'
}
$sha = (Get-FileHash -LiteralPath $orz -Algorithm SHA256).Hash

# ③ ACAF：manifest 在册则复用，否则现场 provision（与 orz_acaf_run.ps1 同形）
# 0by S2（2026-09-28）：**密钥库根＝`<AcafRoot>\keystore`**（与容器侧
# `/etc/orz-acaf/keystore`、载体重建批的 provision 落点、历年实跑命令
# 一致）；`$keystore` 这个导出值一直是对的，**错的是同一段里的 provision
# 调用**——它把密钥库建在了 `$AcafRoot`，于是三处口径错开：新 bin 目录
# （manifest 不在册、现场 provision）下导出根落空 ⇒ 签名器 fail-closed 启动
# 即退（`installation key root is not a directory`）⇒ 每次签票
# `signer_unreachable` ⇒ 票据类工具面全灭
# （`RUN-CLI-6ab93831`／`-6ab93aaa`／`-6ab94e86`）。长期在役目录之所以
# 「看起来没事」，只因 `<AcafRoot>\keystore\` 本来就在册（12/9 起、重建批
# 每次核对四值的那一对）——本缺陷只在**新目录**暴露。provision 落点与导出根
# 必须同指，故在本装配点加门。
$manifest = Join-Path $AcafRoot 'signer-manifest.json'
$keystore = Join-Path $AcafRoot 'keystore'
if (-not (Test-Path -LiteralPath $manifest)) {
    New-Item -ItemType Directory -Path $keystore -Force | Out-Null
    & $provision $keystore $manifest
    Assert-True ($LASTEXITCODE -eq 0) "ACAF provisioning 失败（exit $LASTEXITCODE）"
}
# 0by S2 装配门：密钥库根必须就是 provision 的落点，且签名器启动所需的文件
# 必须真的在册——不满足就**立刻**报出，绝不让它退化成运行期每条票据的
# `signer_unreachable`（那正是本缺陷此前藏身之处）。
Assert-True (Test-Path -LiteralPath $keystore -PathType Container) `
    "ACAF 密钥库根不是目录：$keystore（provision 落点与导出根必须同指；0by S2）"
Assert-True (Test-Path -LiteralPath (Join-Path $keystore 'installation-key.json') -PathType Leaf) `
    "ACAF 密钥库缺 installation-key.json：$keystore（0by S2 装配门）"
$keyBlob = @('installation-key.dpapi', 'installation-key.bin') |
    ForEach-Object { Join-Path $keystore $_ } |
    Where-Object { Test-Path -LiteralPath $_ -PathType Leaf }
Assert-True ($keyBlob.Count -ge 1) `
    "ACAF 密钥库缺受保护密钥文件（installation-key.dpapi／.bin）：$keystore（0by S2 装配门）"

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
# ⑤⑧（0bd）：RLI 显式装配——缺省 on（0bf ① 语义反转：kill switch 才关）；
# -RliOff ⇒ 显式 0。写清单保证会话姿态可核（0bc 轮缺口：靠会话 env 透传、
# 装配清单未显式）。
$env:ORZ_LIF_RLI_SHADOW = if ($RliOff) { '0' } else { '1' }
$protoc = 'D:\tb-eval\.tools\protoc-25.3\bin\protoc.exe'
if (Test-Path -LiteralPath $protoc) { $env:PROTOC = $protoc }
$grokHome = Join-Path $BinDir 'grok-home'
if (Test-Path -LiteralPath $grokHome) { $env:GROK_HOME = $grokHome; $env:GROK_AGENT = '1' }

# ⑤ 装配清单（启动前 checklist）
Write-Host '== 狗粮启动装配清单 =='
Write-Host "cwd        = $ws（断言通过：非 orz 子模块）"
$verDisp = if ($ver -eq 'unknown') { 'unknown' } else { "v$ver" }
Write-Host "carrier    = $orz（$verDisp；sha256 $($sha.Substring(0,12))…）"
# 0by S2：装配清单显式回显 ACAF 的三处口径（此前只看得到 manifest，密钥库根
# 这个曾经出错的量根本不在清单上）。
Write-Host "acaf       = keystore=$keystore / manifest=$manifest"
Write-Host "task       = $taskPath（$($prompt.Length) 字符；$taskEncDisp；读取=显式 UTF-8）"
$rliDisp = if ($RliOff) { 'off（-RliOff kill switch）' } else { 'on（缺省常开）' }
Write-Host "rli        = $rliDisp（ORZ_LIF_RLI_SHADOW=$($env:ORZ_LIF_RLI_SHADOW)）"
Write-Host "env        = MAX_WALLCLOCK=$MaxWallclock / ACAF fail-closed=$(-not $Shadow) / PROTOC=$($env:PROTOC) / GROK_HOME=$($env:GROK_HOME)"
Write-Host 'flags      = --real --allow-write --allow-shell --allow-network -p <task>'
if ($DryRun) { Write-Host '（-DryRun：装配与断言全部通过；未启动。）'; exit 0 }

# ⑥ 启动（日志落 .tmp-*；门禁忽略前缀）
$stamp = Get-Date -Format 'yyyyMMdd-HHmmss'
$log = Join-Path $ws ".tmp-dogfood-$stamp.log"
Write-Host "launching（日志：$log）"
Push-Location -LiteralPath $ws
$prevEap = $ErrorActionPreference
$code = 1
$heartbeat = $null
try {
    # 原生 stderr 经 `2>&1`/`*>&1` 进管道会被包成 ErrorRecord；在
    # $ErrorActionPreference='Stop'（本脚本前段设定）下，PowerShell 5.1 会把它
    # 升级为终止错误 NativeCommandError ⇒ 脚本中止、管道被拆、进程树被收，
    # 且 Tee 日志文件根本不落盘。2026-09-19 狗粮轮实测：RUN-CLI-6aad91f0 跑到
    # 第 10 分钟／72 轮／97 次工具调用，仅因首条 transport WARN（stream idle 5s）
    # 就被打断，journal 停在 seq 776、无任何终态事件。故此处局部降为 Continue：
    # WARN 照常落日志，健康 run 不被误杀。
    # F3 机理澄清（2026-09-19）：本 Tee 管道对载体（Rust stdout 按行 flush）
    # 的日志是按行实时落盘的，不是缓冲病灶；F3 实测的「.tmp 0B 数分钟」病灶在
    # run 内长命令的子进程块缓冲（如 python 无 -u 的 unittest 圆点），属模型
    # 运行时命令纪律（python -u／分段落盘／Start-Process 直写），机械层不做
    # 命令适配（F5/F6 同族边界）。日志静止时以下方心跳侧车判活性，勿以 0B 断死。
    $ErrorActionPreference = 'Continue'
    # F3 活性侧车（2026-09-19 用户裁决「F3 要改」的启动器落点）：每 30s 把运行
    # 时长与载体进程活性（PID/CPU/内存）写入 <log>.live，进程退出即自记终态。
    # 侧车文件同属本 run 产物（ORZ-RUN-SEPARATION-001 归属纪律）。
    $logLive = "$log.live"
    $heartbeat = Start-Job -ScriptBlock {
        param($livePath, $procName)
        $t0 = Get-Date
        while ($true) {
            Start-Sleep -Seconds 30
            $p = Get-Process -Name $procName -ErrorAction SilentlyContinue | Select-Object -First 1
            $state = if ($p) { 'alive pid={0} cpu={1:n1}s ws={2:n0}MB' -f $p.Id, $p.CPU, ($p.WorkingSet64 / 1MB) } else { 'process-exited' }
            Add-Content -LiteralPath $livePath -Value ('[{0:HH:mm:ss}] t+{1:n0}s {2}' -f (Get-Date), ((Get-Date) - $t0).TotalSeconds, $state)
            if (-not $p) { break }
        }
    } -ArgumentList $logLive, 'orz'
    & $orz -p $prompt --real --allow-write --allow-shell --allow-network 2>&1 |
        Tee-Object -FilePath $log
    $code = $LASTEXITCODE
} finally {
    if ($heartbeat) {
        Stop-Job $heartbeat -ErrorAction SilentlyContinue
        Remove-Job $heartbeat -Force -ErrorAction SilentlyContinue
    }
    $ErrorActionPreference = $prevEap
    Pop-Location
}
exit $code
