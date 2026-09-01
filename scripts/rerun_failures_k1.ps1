# THIN-HARNESS-REDESIGN 编译轮后失败题重跑（2026-08-28）：上一轮正式跑分
# （official-r1，58/89）未解出的 31 题，k=1 用新二进制重跑一遍。
# 按 agent 超时档分组（每 harbor run 单一 max_wallclock = timeout - 60），
# gsa 卷自动挂载，.env（ORZ_TOOL_TIMEOUT_SECS=900 / ORZ_STALL_TIMEOUT=360）
# 生效。为规避 PowerShell 原生传参引号问题，每档生成 job config JSON
# 后用 `-c` 启动（镜像官方跑分配置结构）。结果输出到
# D:\tb-eval\jobs-official\official-r2-failures*。
# 用法：powershell -File rerun_failures_k1.ps1 [-Concurrency 2] [-PrintConfig]
param(
  [int]$Concurrency = 2,
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
$TaskDir = 'D:\tb-eval\terminal-bench-2'
$BaseJob = 'official-r2-failures'
$configDir = 'D:\CLI\scripts\configs'
New-Item -ItemType Directory -Path $configDir -Force | Out-Null

$Tasks = @(
  'query-optimize','chess-best-move','make-doom-for-mips','model-extraction-relu-logits',
  'pytorch-model-cli','torch-pipeline-parallelism','raman-fitting','adaptive-rejection-sampler',
  'write-compressor','largest-eigenval','gpt2-codegolf','tune-mjcf','count-dataset-tokens',
  'gcode-to-text','extract-elf',
  'caffe-cifar-10',
  'filter-js-from-html','make-mips-interpreter','protein-assembly','dna-insert','path-tracing',
  'rstan-to-pystan','extract-moves-from-video','path-tracing-reverse','mteb-retrieve','dna-assembly',
  'circuit-fibsqrt','mteb-leaderboard','video-processing','train-fasttext',
  'build-pov-ray'
)

function Get-TaskTimeout([string]$name) {
  $toml = Join-Path $TaskDir "$name\task.toml"
  if (Test-Path -LiteralPath $toml) {
    $raw = Get-Content -LiteralPath $toml -Encoding UTF8 -Raw
    $m = [regex]::Match($raw, '(?ms)\[agent\]\s*timeout_sec\s*=\s*(\d+)')
    if ($m.Success) { return [int]$m.Groups[1].Value }
  }
  Write-Warning "no [agent] timeout_sec for $name; falling back to 900"
  return 900
}

$classes = @{}
foreach ($t in $Tasks) {
  $to = Get-TaskTimeout $t
  if (-not $classes.ContainsKey($to)) { $classes[$to] = [System.Collections.Generic.List[string]]::new() }
  $classes[$to].Add($t)
}

$WALLCLOCK_MARGIN = 60
$lastExit = 0
foreach ($to in ($classes.Keys | Sort-Object)) {
  $clsTasks = $classes[$to]
  $wall = [Math]::Max(1, $to - $WALLCLOCK_MARGIN)
  $jobName = if ($classes.Count -eq 1) { $BaseJob } else { "$BaseJob-$($to)s" }
  $volDir = Join-Path $volRoot $jobName
  if (-not $PrintConfig) { New-Item -ItemType Directory -Path $volDir -Force | Out-Null }
  $mount = [ordered]@{
    type = 'bind'
    source = ($volDir -replace '\\', '/')
    target = '/orz-gsa'
  }
  $config = [ordered]@{
    job_name = $jobName
    jobs_dir = 'D:\tb-eval\jobs-official'
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
        gsa_volume = $volDir
        max_wallclock = "$wall"
      }
    })
    datasets = @([ordered]@{
      name = 'terminal-bench/terminal-bench-2-1'
      ref = 'sha256:7d7bdc1cbedad549fc1140404bd4dc45e5fd0ea7c4186773687d177ad3a0699a'
      task_names = @($clsTasks | ForEach-Object { "terminal-bench/$_" })
    })
  }
  $configPath = Join-Path $configDir "$jobName-config.json"
  $json = $config | ConvertTo-Json -Depth 6
  # 无 BOM UTF-8（PowerShell 5.1 Set-Content 会带 BOM，Python json 会拒）。
  try {
    [System.IO.File]::WriteAllText($configPath, $json, [System.Text.UTF8Encoding]::new($false))
  } catch {
    Write-Host ("    WRITE-ERR: " + $_.Exception.GetType().FullName + " : " + $_.Exception.Message)
  }
  Write-Host ("    jsonType=" + $json.GetType().FullName + " jsonLen=" + $json.Length)
  Write-Host "    config=$configPath exists=$(Test-Path -LiteralPath $configPath)"
  Write-Host "==> Rerun [$($to)s] : $($clsTasks -join ' ') | max_wallclock=$wall | volume=$volDir"
  $hbArgs = @(
    '-c', $configPath,
    '--env-file=D:\tb-eval\.env',
    '-k', '1',
    '--n-concurrent', "$Concurrency"
  )
  if ($PrintConfig) { $hbArgs += '--print-config' } else { $hbArgs += '-y' }
  & $Harbor run @hbArgs
  $lastExit = $LASTEXITCODE
  if (-not $PrintConfig -and $lastExit -ne 0) { break }
}
exit $lastExit
