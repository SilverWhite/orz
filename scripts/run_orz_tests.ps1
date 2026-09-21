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
# 用法（从任意目录）：
#   powershell -File scripts\run_orz_tests.ps1 test -p orz-loop --lib -j 2
$orzRoot = Join-Path (Split-Path -Parent $PSScriptRoot) 'orz'
if (-not (Test-Path $orzRoot)) { Write-Error "orz workspace not found at $orzRoot"; exit 2 }
foreach ($k in @('ORZ_ACAF_FAIL_CLOSED','ORZ_ACAF_MANIFEST','ORZ_ACAF_KEYSTORE','ORZ_ACAF_BINARY','ORZ_LIF_RLI_SHADOW','GROK_HOME','GROK_AGENT')) {
    Remove-Item "Env:$k" -ErrorAction SilentlyContinue
}
if (-not $env:RUST_MIN_STACK) { $env:RUST_MIN_STACK = '67108864' }
Push-Location $orzRoot
try {
    & cargo @args
    exit $LASTEXITCODE
} finally {
    Pop-Location
}