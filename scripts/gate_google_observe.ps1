# 0k-5 Google gate observation (2026-08-30, DUAL 3.2 / research 8.4).
# Small retrieval-heavy batch, k=1, local_browser lane (orz run script
# always passes --retrieval-mode local_browser), container chromium
# injection enabled (eval_browser=true). Observes Google-first SERP gates
# (CAPTCHA/429/page structure) + pass rate / wallclock / citation binding.
#
# Usage: powershell -File gate_google_observe.ps1 [-PrintConfig]
# Preconditions:
#   1. ORZ_DEEPSEEK_API_KEY present in D:/tb-eval/.env
#   2. D:/tb-eval/orz-linux/orz = 2026-08-30 S3 rebuild (b604773)
#   3. tb_agents/orz.py has eval_browser flag (SHA256 047830C8...)
#   4. Docker Desktop engine running (apt chromium / snapshot injection)
param([switch]$PrintConfig)
$ErrorActionPreference = 'Stop'
$env:PYTHONPATH = 'D:\tb-eval'

$Harbor = 'D:\tb-eval\venv\Scripts\harbor.exe'
$Orz = 'D:/tb-eval/orz-linux/orz'
$Model = 'deepseek-v4-flash'
$JobsDir = 'D:\tb-eval\jobs-gate'
$volRoot = 'D:\tb-eval\gsa-volumes'
$configDir = 'D:\CLI\scripts\configs'

# DUAL 3.2 task set: historical timeout / retrieval-dependent tasks, k=1.
$Tasks = @(
  'mteb-leaderboard',
  'path-tracing-reverse',
  'rstan-to-pystan',
  'configure-git-webserver',
  'mteb-retrieve'
)

$jobName = 'gate-google-20260830-1'
$volDir = Join-Path $volRoot $jobName
$resultPath = Join-Path $JobsDir "$jobName\result.json"
if (Test-Path -LiteralPath $resultPath) {
  $raw = Get-Content -LiteralPath $resultPath -Encoding UTF8 -Raw
  if ($raw -match '"finished_at": "') {
    Write-Host "==> $jobName already complete (finished result.json present), skipping"
    exit 0
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
      # 0k-5: container chromium injection (apt chromium / snapshot fallback)
      eval_browser = 'true'
    }
  })
  datasets = @([ordered]@{
    name = 'terminal-bench/terminal-bench-2-1'
    ref = 'sha256:7d7bdc1cbedad549fc1140404bd4dc45e5fd0ea7c4186773687d177ad3a0699a'
    task_names = @($Tasks | ForEach-Object { "terminal-bench/$_" })
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
Write-Host "==> $jobName : $($Tasks.Count) tasks | volume=$volDir"
& $Harbor @hbArgs
if ($LASTEXITCODE -ne 0) { throw "harbor run failed for $jobName (exit $LASTEXITCODE)" }
