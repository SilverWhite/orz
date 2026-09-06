$ErrorActionPreference = 'Continue'
$env:PYTHONPATH = 'D:/tb-eval'

# Final smoke (official-standard settings, k=1, no upload) - 5 tasks
$logDir   = 'D:\tb-eval\jobs-official'
$harbor   = 'D:\tb-eval\venv\Scripts\harbor.exe'
$config   = 'D:\CLI\_final_smoke_2026-08-25_config.json'
$stdout   = Join-Path $logDir 'final-smoke-2026-08-25-run.log'
$stderr   = Join-Path $logDir 'final-smoke-2026-08-25-run.err.log'
$exitFile = Join-Path $logDir 'final-smoke-2026-08-25-exit.txt'

New-Item -ItemType Directory -Force -Path $logDir | Out-Null

try {
  & $harbor run --config $config --env-file D:/tb-eval/.env -y *> $stdout
  $code = $LASTEXITCODE
} catch {
  $code = 1
  $_ | Out-String | Out-File -Encoding utf8 $stderr
}

"EXIT=$code" | Out-File -Encoding utf8 $exitFile
exit $code
