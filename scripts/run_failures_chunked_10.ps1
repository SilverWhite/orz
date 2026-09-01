# THIN-HARNESS-REDESIGN R1/R2 编译轮后失败题重跑（2026-08-28）——
# 上次 89 题官方跑分（official-r1，k=1，58/89）未解出的 31 题，
# 用新二进制（orz 6cc8586，S3 重建 07:00 HKT）k=1 再跑一轮，10 题一批。
#
# 参数与 run_official_chunked_10.sh 一致（k=1、每批 10、不加 max_wallclock
# 覆盖、任务按官方 agent 超时执行），任务表=31 道错题、job 前缀=
# official-r2-failures，且**不上传**（本地验证重跑，leaderboard 合并另行
# 决定）。已含 result.json（finished_at 非空）的 job 自动跳过，可断点续跑。
#
# 用法：powershell -File run_failures_chunked_10.ps1 [-Chunk 1|2|3|4|all]
#       [-PrintConfig]（仅生成并打印 config JSON，不启动 job）
#
# Preconditions:
#   1. ORZ_DEEPSEEK_API_KEY present in D:/tb-eval/.env
#   2. D:/tb-eval/orz-linux/orz = 新二进制（S3 重建产物）
#   3. Output goes to D:/tb-eval/jobs-official
param(
  [string]$Chunk = 'all',
  [switch]$PrintConfig
)
$ErrorActionPreference = 'Stop'
$env:PYTHONPATH = 'D:\tb-eval'

$Harbor = 'D:\tb-eval\venv\Scripts\harbor.exe'
$Orz = 'D:/tb-eval/orz-linux/orz'
$Dataset = 'terminal-bench/terminal-bench-2-1@sha256:7d7bdc1cbedad549fc1140404bd4dc45e5fd0ea7c4186773687d177ad3a0699a'
$Model = 'deepseek-v4-flash'
$JobsDir = 'D:\tb-eval\jobs-official'
$volRoot = 'D:\tb-eval\gsa-volumes'
$configDir = 'D:\CLI\scripts\configs'

$Tasks = @(
  'query-optimize','chess-best-move','make-doom-for-mips','model-extraction-relu-logits',
  'pytorch-model-cli','torch-pipeline-parallelism','raman-fitting','adaptive-rejection-sampler',
  'write-compressor','largest-eigenval',
  'gpt2-codegolf','tune-mjcf','count-dataset-tokens','gcode-to-text','extract-elf',
  'caffe-cifar-10','filter-js-from-html','make-mips-interpreter','protein-assembly','dna-insert',
  'path-tracing','rstan-to-pystan','extract-moves-from-video','path-tracing-reverse','mteb-retrieve',
  'dna-assembly','circuit-fibsqrt','mteb-leaderboard','video-processing','train-fasttext',
  'build-pov-ray'
)

function Invoke-Chunk {
  param([string]$jobName, [string[]]$tasks)
  $volDir = Join-Path $volRoot $jobName
  $resultPath = Join-Path $JobsDir "$jobName\result.json"
  if (Test-Path -LiteralPath $resultPath) {
    $raw = Get-Content -LiteralPath $resultPath -Encoding UTF8 -Raw
    if ($raw -match '"finished_at": "') {
      Write-Host "==> $jobName already complete (finished result.json present), skipping"
      return
    }
  }
  New-Item -ItemType Directory -Path $volDir -Force | Out-Null
  New-Item -ItemType Directory -Path $configDir -Force | Out-Null
  $volFwd = $volDir -replace '\\', '/'
  $mount = [ordered]@{
    type = 'bind'
    source = $volFwd
    target = '/orz-gsa'
  }
  $config = [ordered]@{
    job_name = $jobName
    jobs_dir = $JobsDir
    environment = [ordered]@{
      type = 'docker'
      mounts = @($mount)
    }
    agents = @([ordered]@{
      name = 'tb_agents.orz:Orz'
      model_name = $Model
      kwargs = [ordered]@{
        orz_binary = $Orz
        model_id = $Model
        gsa_volume = $volFwd
      }
    })
    datasets = @([ordered]@{
      name = 'terminal-bench/terminal-bench-2-1'
      ref = 'sha256:7d7bdc1cbedad549fc1140404bd4dc45e5fd0ea7c4186773687d177ad3a0699a'
      task_names = @($tasks | ForEach-Object { "terminal-bench/$_" })
    })
  }
  $configPath = Join-Path $configDir "$jobName-config.json"
  $json = $config | ConvertTo-Json -Depth 6
  [System.IO.File]::WriteAllText($configPath, $json, [System.Text.UTF8Encoding]::new($false))
  if ($PrintConfig) {
    $hbArgs = @('run', '-c', $configPath, '--print-config')
  } else {
    $hbArgs = @(
      'run', '-c', $configPath,
      '--env-file', 'D:/tb-eval/.env',
      '-k', '1',
      '-y'
    )
  }
  Write-Host "==> $jobName : $($tasks.Count) tasks | volume=$volDir"
  & $Harbor @hbArgs
  if ($LASTEXITCODE -ne 0) { throw "harbor run failed for $jobName (exit $LASTEXITCODE)" }
}

$chunks = @()
for ($i = 0; $i -lt $Tasks.Count; $i += 10) {
  $end = [Math]::Min($i + 9, $Tasks.Count - 1)
  $chunks += ,@($Tasks[$i..$end])
}

if ($Chunk -eq 'all') {
  for ($c = 1; $c -le $chunks.Count; $c++) {
    Invoke-Chunk "official-r2-failures-c$c" $chunks[$c - 1]
  }
} else {
  $n = [int]$Chunk
  if ($n -lt 1 -or $n -gt $chunks.Count) {
    throw "invalid chunk '$Chunk' (valid: 1..$($chunks.Count) or 'all')"
  }
  Invoke-Chunk "official-r2-failures-c$n" $chunks[$n - 1]
}
