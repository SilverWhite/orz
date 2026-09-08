# 20 题官方复跑（r3）— 一题一作业顺序执行器（2026-09-08）
# 用途：harbor 整批作业在网络抖动时会在题边界中止并整批重跑；本脚本将每题
# 拆为独立 job（official-r3-unsolved20-<task>），完成即跳过、失败单独重试，
# 避免整批归零。口径与配置 official-r3-unsolved20-c1/c2 一致：k=1、
# deepseek-v4-flash、eval_browser=true、官方数据集 pin、当前源 0.3.2+0q
# 二进制 D:/tb-eval/orz-linux/orz。dna-assembly 已在整批轮取得有效官方结果
# （reward 0.0，AgentTimeout），不再重复跑（k=1 纪律）。
$ErrorActionPreference = 'Stop'
$env:PYTHONPATH = 'D:/tb-eval'
$Harbor = 'D:\tb-eval\venv\Scripts\harbor.exe'
$Dataset = 'terminal-bench/terminal-bench-2-1@sha256:7d7bdc1cbedad549fc1140404bd4dc45e5fd0ea7c4186773687d177ad3a0699a'
$JobsDir = 'D:/tb-eval/jobs-official'
$VolRoot = 'D:/tb-eval/gsa-volumes'
$Binary = 'D:/tb-eval/orz-linux/orz'
$Model = 'deepseek-v4-flash'
$Summary = Join-Path $JobsDir 'official-r3-unsolved20-per-task-summary.log'

$Tasks = @(
  'adaptive-rejection-sampler',
  'count-dataset-tokens',
  'dna-insert',
  'extract-elf',
  'extract-moves-from-video',
  'filter-js-from-html',
  'gcode-to-text',
  'gpt2-codegolf',
  'make-doom-for-mips',
  'make-mips-interpreter',
  'model-extraction-relu-logits',
  'mteb-leaderboard',
  'path-tracing',
  'path-tracing-reverse',
  'protein-assembly',
  'raman-fitting',
  'train-fasttext',
  'tune-mjcf',
  'write-compressor'
)

function Test-JobComplete {
  param([string]$TaskName)
  $jobDir = Join-Path $JobsDir "official-r3-unsolved20-$TaskName"
  $rj = Join-Path $jobDir 'result.json'
  if (-not (Test-Path -LiteralPath $rj)) { return $false }
  $raw = Get-Content -LiteralPath $rj -Raw -Encoding UTF8
  return $raw -match '"finished_at":\s*"'
}

function Invoke-OneTask {
  param([string]$TaskName)
  $jobName = "official-r3-unsolved20-$TaskName"
  $volDir = Join-Path $VolRoot $jobName
  New-Item -ItemType Directory -Path $volDir -Force | Out-Null
  $mounts = '[{"type":"bind","source":"' + ($volDir -replace '\\','/') + '","target":"/orz-gsa"}]'
  $runArgs = @('run','-d',$Dataset,'-i',("terminal-bench/$TaskName"),'-n','1','-r','3',
    '-a','tb_agents.orz:Orz','-m',$Model,
    '--ak',"orz_binary=$Binary",
    '--ak',"model_id=$Model",
    '--ak',"gsa_volume=$volDir",
    '--ak','eval_browser=true',
    '--mounts',$mounts,
    '--env-file','D:/tb-eval/.env',
    '--job-name',$jobName,
    '-o',$JobsDir,
    '-k','1','-y')
  $console = Join-Path $JobsDir "$jobName-console.log"
  & $Harbor @runArgs *> $console
  return $LASTEXITCODE
}

$ts = Get-Date -Format 'yyyy-MM-dd HH:mm:ss'
Add-Content -LiteralPath $Summary -Encoding utf8 "== $ts per-task run start (19 tasks; dna-assembly kept from batch run) =="

$pending = @($Tasks)
for ($pass = 1; $pass -le 3 -and $pending.Count -gt 0; $pass++) {
  Add-Content -LiteralPath $Summary -Encoding utf8 "pass $pass : $($pending.Count) pending"
  $next = @()
  foreach ($t in $pending) {
    if (Test-JobComplete $t) {
      Add-Content -LiteralPath $Summary -Encoding utf8 "SKIP (complete) $t"
      continue
    }
    Add-Content -LiteralPath $Summary -Encoding utf8 "START $t"
    $code = Invoke-OneTask $t
    if ($code -eq 0 -and (Test-JobComplete $t)) {
      Add-Content -LiteralPath $Summary -Encoding utf8 "DONE $t exit=$code"
    } else {
      Add-Content -LiteralPath $Summary -Encoding utf8 "FAIL $t exit=$code"
      $next += $t
    }
  }
  $pending = $next
}

Add-Content -LiteralPath $Summary -Encoding utf8 "== finished; unresolved: $($pending -join ', ') =="
