$env:Path = [Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [Environment]::GetEnvironmentVariable('Path','User')
$env:PYTHONPATH = 'C:\s4'
foreach ($f in @('orz.exe', 'orz-signer.exe', 'orz-acaf-provision.exe')) {
    $orzSrc = Join-Path 'C:\s4\tools' $f
    $orzDst = Join-Path 'C:\Program Files\orz' $f
    if (Test-Path -LiteralPath $orzSrc) {
        Copy-Item -LiteralPath $orzSrc -Destination $orzDst -Force
        $h1 = (Get-FileHash -LiteralPath $orzSrc -Algorithm SHA256).Hash
        $h2 = (Get-FileHash -LiteralPath $orzDst -Algorithm SHA256).Hash
        Write-Output "AGENT_ORZ_SYNC $f ok=$($h1 -eq $h2)"
    }
}
$agentRun = 'C:\s4\_windows_high_nist\run\run_agent_arm.ps1'
$ws = 'C:\workspace\agent-tb2.1-f2-dryrun'
$setDir = 'C:\s4\_windows_high_nist\agent-tasks-tb2.1'
$ids = 'make-doom-for-mips'
$arm = 'high-nist'
if (Test-Path -LiteralPath $ws) {
    Remove-Item -LiteralPath $ws -Recurse -Force -ErrorAction SilentlyContinue
}
New-Item -ItemType Directory -Path $ws -Force | Out-Null
Write-Output "===== AGENT LIVE RUN arm=$arm taskSet=$setDir ids=$ids dryRun=True ====="
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File $agentRun -Arm $arm -Workspace $ws `
    -TasksDir $setDir -TaskIds $ids -ResultPath (Join-Path $ws "agent-baseline-$arm.json") -DryRun 2>&1 |
    ForEach-Object { Write-Output "AGENTLIVE: $_" }
$code = $LASTEXITCODE
Write-Output "AGENT_LIVE_EXIT=$code"
if (Test-Path -LiteralPath (Join-Path $ws "agent-baseline-$arm.json")) {
    Get-Content -LiteralPath (Join-Path $ws "agent-baseline-$arm.json") -Raw
}
exit $code
