<#
    S4 VM restricted-token spawn comparison (host-side elevated):
    spawns cmd.exe and powershell.exe under the non-admin restricted token
    via the sandbox runner, reports exit codes.
    ASCII only.
#>
[CmdletBinding()]
param(
    [string]$OutFile = 'D:\CLI\_windows_high_nist\vm-diag-spawn-result.txt'
)

$ErrorActionPreference = 'Stop'
$lines = New-Object System.Collections.Generic.List[string]
$runner = 'D:\CLI\scripts\s4_vm_run_elev.ps1'

try {
    $sec = ConvertTo-SecureString '123456' -AsPlainText -Force
    $cred = New-Object System.Management.Automation.PSCredential('HL', $sec)
    $sess = New-PSSession -VMName 'win-s4' -Credential $cred
    Copy-Item -LiteralPath 'D:\CLI\assurance\windows_sandbox.py' `
        -Destination 'C:\s4\assurance\windows_sandbox.py' -ToSession $sess -Force
    $srcHash = (Get-FileHash -LiteralPath 'D:\CLI\assurance\windows_sandbox.py' -Algorithm SHA256).Hash
    $dstHash = Invoke-Command -Session $sess -ScriptBlock {
        (Get-FileHash -LiteralPath 'C:\s4\assurance\windows_sandbox.py' -Algorithm SHA256).Hash
    }
    $lines.Add("SYNC_WINDOWS_SANDBOX=$($srcHash -eq $dstHash)")
    Remove-PSSession -Session $sess -ErrorAction SilentlyContinue
} catch {
    $lines.Add("SYNC_ERR: $($_.Exception.Message)")
}

$body = @'
$env:Path = [Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [Environment]::GetEnvironmentVariable('Path','User')
$py = 'C:\Program Files\Python312\python.exe'
New-Item -ItemType Directory -Path 'C:\s4\results' -Force | Out-Null
New-Item -ItemType Directory -Path 'C:\s4\results\ws-non-admin' -Force | Out-Null
New-Item -ItemType Directory -Path 'C:\s4\diag-ws' -Force | Out-Null
$marker = 'C:\s4\diag-ws\.assurance-p2-disposable.json'
@{ schema_version = '0.1.0-draft'; purpose = 'windows-native-sandbox-probe'; allow_container_write_probe = $true } |
    ConvertTo-Json | Set-Content -LiteralPath $marker -Encoding ascii
$cmdObs = 'C:\s4\results\diag-cmd-obs.json'
& $py C:\s4\scripts\run_windows_native_sandbox_command.py --workspace C:\s4\diag-ws --arm non-admin --timeout 60 --output $cmdObs --command cmd.exe /c exit 42
Write-Output "CMD_RUNNER_EXIT=$LASTEXITCODE"
if (Test-Path -LiteralPath $cmdObs) {
    $j = Get-Content -LiteralPath $cmdObs -Raw | ConvertFrom-Json
    Write-Output "CMD_OBS outcome=$($j.outcome) exit=$($j.process.exit_code)"
    if ($j.diagnostics) { $j.diagnostics | ForEach-Object { Write-Output "CMD_DIAG: $_" } }
}
$psObs = 'C:\s4\results\diag-ps-obs.json'
& $py C:\s4\scripts\run_windows_native_sandbox_command.py --workspace C:\s4\diag-ws --arm non-admin --timeout 60 --output $psObs --command powershell.exe -NoProfile -Command exit 42
Write-Output "PS_RUNNER_EXIT=$LASTEXITCODE"
if (Test-Path -LiteralPath $psObs) {
    $j = Get-Content -LiteralPath $psObs -Raw | ConvertFrom-Json
    Write-Output "PS_OBS outcome=$($j.outcome) exit=$($j.process.exit_code)"
    if ($j.diagnostics) { $j.diagnostics | ForEach-Object { Write-Output "PS_DIAG: $_" } }
}
$ctlObs = 'C:\s4\results\diag-ctl-obs.json'
& $py C:\s4\scripts\run_windows_native_sandbox_command.py --workspace C:\s4\diag-ws --arm control --timeout 60 --output $ctlObs --command cmd.exe /c exit 42
Write-Output "CTL_RUNNER_EXIT=$LASTEXITCODE"
if (Test-Path -LiteralPath $ctlObs) {
    $j = Get-Content -LiteralPath $ctlObs -Raw | ConvertFrom-Json
    Write-Output "CTL_OBS outcome=$($j.outcome) exit=$($j.process.exit_code)"
    if ($j.diagnostics) { $j.diagnostics | ForEach-Object { Write-Output "CTL_DIAG: $_" } }
}
Write-Output '===== FULL non-admin PROBE ====='
& C:\s4\_windows_high_nist\run\run_enforcement_probe.ps1 -Arm non-admin -Workspace C:\s4\results\ws-non-admin -ResultPath C:\s4\results\enforcement-probe-non-admin.json
Write-Output "PROBE_EXIT=$LASTEXITCODE"
if (Test-Path -LiteralPath 'C:\s4\results\enforcement-probe-non-admin.json') {
    Get-Content -LiteralPath 'C:\s4\results\enforcement-probe-non-admin.json' -Raw
}
'@

$bodyFile = 'D:\CLI\_windows_high_nist\job-diag-spawn-body.ps1'
Set-Content -LiteralPath $bodyFile -Value $body -Encoding utf8
try {
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $runner `
        -JobBodyFile $bodyFile -OutFile $OutFile -TimeoutSeconds 400
    if ($LASTEXITCODE -ne 0) { throw "diag-spawn job failed (exit $LASTEXITCODE)" }
}
catch {
    $lines.Add("ERROR: $($_.Exception.Message)")
    $lines | Set-Content -LiteralPath $OutFile -Encoding utf8
}
