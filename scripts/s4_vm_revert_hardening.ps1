<#
    Host-side elevated helper: revert the live high-nist hardening on win-s4
    WITHOUT restoring the baseline checkpoint (keeps installed software,
    profiles and credentials intact).  Runs apply_hardening -Arm high-nist
    -Revert inside the guest as a SYSTEM job via s4_vm_run_elev.ps1.

    ASCII only.
#>
[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$outDir = 'D:\CLI\_windows_high_nist'
$bodyFile = Join-Path $outDir 'job-revert-hn-body.ps1'
$resFile = Join-Path $outDir 'vm-revert-hn-result.txt'
$runner = 'D:\CLI\scripts\s4_vm_run_elev.ps1'

$body = @'
$env:Path = [Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [Environment]::GetEnvironmentVariable('Path','User')
$hard = 'C:\s4\_windows_high_nist\hardening\apply_hardening.ps1'
Write-Output '===== REVERT HIGH-NIST ====='
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File $hard -Arm high-nist -RunUser AgentUser -WorkspaceRoot C:\workspace -EnableAppLocker -Revert 2>&1 |
    ForEach-Object { Write-Output "RVH: $_" }
$code = $LASTEXITCODE
Write-Output "REVERT_HN_EXIT=$code"
exit $code
'@

Set-Content -LiteralPath $bodyFile -Value $body -Encoding utf8
Remove-Item -LiteralPath $resFile -Force -ErrorAction SilentlyContinue
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File $runner `
    -JobBodyFile $bodyFile -OutFile $resFile -TimeoutSeconds 540
$runnerExit = $LASTEXITCODE

$ok = $false
$revertExit = -1
if (Test-Path -LiteralPath $resFile) {
    $resText = @(Get-Content -LiteralPath $resFile -Encoding UTF8)
    foreach ($line in $resText) {
        Write-Output $line
    }
    $hit = $resText | Where-Object { $_ -match '^REVERT_HN_EXIT=(-?\d+)$' } | Select-Object -Last 1
    if ($hit -and $hit -match '^REVERT_HN_EXIT=(-?\d+)$') {
        $revertExit = [int]$Matches[1]
    }
    $ok = ($runnerExit -eq 0) -and ($revertExit -eq 0)
}
Write-Output "REVERT_OK=$ok"
if (-not $ok) {
    exit 1
}
exit 0
