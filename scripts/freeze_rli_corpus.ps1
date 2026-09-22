<#
.SYNOPSIS
  RLI 对拍语料冻结（0bd ⑪）：把 live runs 复制为冻结副本＋sha256 清单；此后
  的严格对拍一律对冻结副本做（避免归档漂移——0be 实测 live 两轮间 16→15
  runs、行数 4876→4063，跨时对比不可严格比较）。
.DESCRIPTION
  来源默认 D:\CLI\.gsa\runs；副本默认 D:\tb-eval\analysis\frozen\rli-corpus-<stamp>。
  每条 run 复制 events.jsonl，写 MANIFEST.json（逐文件 sha256/字节/事件行数/
  源 mtime）与 README.txt（口径说明）。official 137 语料已是冻结件
  （D:\tb-eval\jobs-official），不属本脚本范围。
.PARAMETER Source
  runs 根目录（缺省 D:\CLI\.gsa\runs）。
.PARAMETER Out
  冻结副本目录（缺省 D:\tb-eval\analysis\frozen\rli-corpus-<yyyyMMdd-HHmmss>）。
.PARAMETER Runs
  指定 run 名（如 RUN-CLI-6ab17325）；缺省＝Source 下全部。
.PARAMETER Latest
  只取最近 N 个 run（按 LastWriteTime 降序）。
.EXAMPLE
  powershell -File scripts\freeze_rli_corpus.ps1 -Latest 20
  # 之后：rli-forecast-probe <Out>\runs（0bd ⑩ 收编后的 example 同用法）
#>
param(
    [string]$Source = 'D:\CLI\.gsa\runs',
    [string]$Out,
    [string[]]$Runs,
    [int]$Latest = 0
)
$ErrorActionPreference = 'Stop'
if (-not (Test-Path -LiteralPath $Source)) { Write-Error "source not found: $Source"; exit 2 }
if (-not $Out) { $Out = Join-Path 'D:\tb-eval\analysis\frozen' ("rli-corpus-" + (Get-Date -Format 'yyyyMMdd-HHmmss')) }

$dirs = Get-ChildItem -LiteralPath $Source -Directory | Sort-Object LastWriteTime -Descending
if ($Runs) { $dirs = @($dirs | Where-Object { $Runs -contains $_.Name }) }
elseif ($Latest -gt 0) { $dirs = @($dirs | Select-Object -First $Latest) }
else { $dirs = @($dirs) }
if ($dirs.Count -eq 0) { Write-Error 'no runs selected'; exit 2 }

New-Item -ItemType Directory -Path $Out -Force | Out-Null
$runsDir = Join-Path $Out 'runs'
New-Item -ItemType Directory -Path $runsDir -Force | Out-Null

$entries = @()
foreach ($d in $dirs) {
    $ev = Join-Path $d.FullName 'events.jsonl'
    if (-not (Test-Path -LiteralPath $ev)) { Write-Warning "skip $($d.Name)：无 events.jsonl"; continue }
    $dst = Join-Path $runsDir $d.Name
    New-Item -ItemType Directory -Path $dst -Force | Out-Null
    Copy-Item -LiteralPath $ev -Destination (Join-Path $dst 'events.jsonl') -Force
    $hash = (Get-FileHash -LiteralPath (Join-Path $dst 'events.jsonl') -Algorithm SHA256).Hash
    $len = (Get-Item (Join-Path $dst 'events.jsonl')).Length
    $lines = (Get-Content -LiteralPath (Join-Path $dst 'events.jsonl') -Encoding UTF8 | Measure-Object -Line).Lines
    $entries += [pscustomobject]@{
        run = $d.Name; sha256 = $hash; bytes = $len; lines = $lines
        source_mtime = $d.LastWriteTime.ToString('o')
    }
    Write-Host ("frozen {0}  sha256 {1}…  {2} bytes / {3} lines" -f $d.Name, $hash.Substring(0, 12), $len, $lines)
}
$manifest = [pscustomobject]@{
    schema    = 'rli-corpus-freeze-v1'
    frozen_at = (Get-Date).ToString('o')
    frozen_by = 'scripts/freeze_rli_corpus.ps1（0bd ⑪）'
    source    = (Resolve-Path -LiteralPath $Source).Path
    runs      = $entries
}
$manifest | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath (Join-Path $Out 'MANIFEST.json') -Encoding UTF8
@(
    'RLI 对拍语料冻结副本（0bd ⑪ 冻结语料纪律）。'
    "来源: $Source"
    "冻结时间: $((Get-Date).ToString('o'))"
    '口径: 严格对拍（跨时对比）一律使用本副本；live 目录继续变动不影响本副本。'
    '用法: rli-forecast-probe <本目录>\runs；或按 MANIFEST.json 逐文件复算 sha256。'
    "run 数: $($entries.Count)；清单: MANIFEST.json"
) | Set-Content -LiteralPath (Join-Path $Out 'README.txt') -Encoding UTF8
Write-Host "== frozen $($entries.Count) runs -> $Out =="
