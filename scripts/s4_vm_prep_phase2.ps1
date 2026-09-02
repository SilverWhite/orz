<#
    S4 VM prep phase 2 driver (host-side elevated):
      A) relocate orz trio to C:\Program Files\orz
      B) install Python machine-wide to C:\Program Files\Python312
      C) repoint ORZ_ACAF_BINARY + verify signer hash + python
      D) baseline checkpoint win-s4
      E) control-arm enforcement probe (pre-hardening baseline)
    ASCII only.
#>
[CmdletBinding()]
param(
    [string]$OutFile = 'D:\CLI\_windows_high_nist\vm-prep-phase2-result.txt'
)

$ErrorActionPreference = 'Stop'
$lines = New-Object System.Collections.Generic.List[string]
$runner = 'D:\CLI\scripts\s4_vm_run_elev.ps1'

function Invoke-GuestJob {
    param([string]$Name, [string]$Body, [int]$TimeoutSeconds = 1200)
    $res = Join-Path 'D:\CLI\_windows_high_nist' ("job-$Name-result.txt")
    $bodyFile = Join-Path 'D:\CLI\_windows_high_nist' ("job-$Name-body.ps1")
    Set-Content -LiteralPath $bodyFile -Value $Body -Encoding utf8
    Remove-Item -LiteralPath $res -Force -ErrorAction SilentlyContinue
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $runner `
        -JobBodyFile $bodyFile -OutFile $res -TimeoutSeconds $TimeoutSeconds
    if ($LASTEXITCODE -ne 0) {
        throw "guest job $Name runner failed (exit $LASTEXITCODE)"
    }
    $lines.Add("===== JOB $Name =====")
    foreach ($l in (Get-Content -LiteralPath $res -Encoding UTF8)) {
        $lines.Add($l)
    }
}

try {
    $bodyA = @'
$src = 'C:\orz'
$dst = 'C:\Program Files\orz'
New-Item -ItemType Directory -Path $dst -Force | Out-Null
foreach ($f in @('orz.exe', 'orz-signer.exe', 'orz-acaf-provision.exe')) {
    $s = Join-Path $src $f
    $d = Join-Path $dst $f
    if (Test-Path -LiteralPath $d) { Remove-Item -LiteralPath $d -Force }
    if (-not (Test-Path -LiteralPath $s)) { Write-Output "MISSING $s"; continue }
    Move-Item -LiteralPath $s -Destination $d -Force
    Write-Output "MOVED $f size=$((Get-Item -LiteralPath $d).Length)"
}
'@
    Invoke-GuestJob -Name 'relocate-orz' -Body $bodyA

    $bodyB = @'
$installer = 'C:\s4\tools\python-3.12.10-amd64.exe'
if (-not (Test-Path -LiteralPath $installer)) {
    Write-Output 'INSTALLER missing'
    exit 2
}
$installed = $false
for ($a = 1; $a -le 10; $a++) {
    try {
        $p = Start-Process -FilePath $installer `
            -ArgumentList @('/quiet', 'InstallAllUsers=1', 'PrependPath=1', 'Include_test=0', 'Include_launcher=1', 'AssociateFiles=0', 'Shortcuts=0', 'TargetDir=C:\Program Files\Python312') `
            -Wait -PassThru -ErrorAction Stop
        Write-Output "INSTALL attempt=$a exit=$($p.ExitCode)"
        if ($p.ExitCode -eq 0) { $installed = $true; break }
    } catch {
        Write-Output "INSTALL attempt=$a err=$($_.Exception.Message)"
        Start-Sleep -Seconds 3
    }
}
if (-not $installed) { exit 1 }
'@
    Invoke-GuestJob -Name 'install-python-allusers' -Body $bodyB

    $bodyC = @'
[Environment]::SetEnvironmentVariable('ORZ_ACAF_BINARY', 'C:\Program Files\orz\orz-signer.exe', 'Machine')
Write-Output "BINARY=$([Environment]::GetEnvironmentVariable('ORZ_ACAF_BINARY','Machine'))"
$manifestPath = 'C:\Users\HL\AppData\Local\orz\acaf\signer-manifest.json'
$manifest = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
$hash = (Get-FileHash -LiteralPath 'C:\Program Files\orz\orz-signer.exe' -Algorithm SHA256).Hash
Write-Output "SIGNER_HASH=$hash"
Write-Output "MANIFEST_HASH=$($manifest.binary_sha256)"
if ($hash -ne $manifest.binary_sha256) { Write-Output 'SIGNER_HASH_MISMATCH'; exit 1 }
Write-Output 'SIGNER_HASH_MATCH=yes'
$env:Path = [Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [Environment]::GetEnvironmentVariable('Path','User')
$py = & python -c "import sys; print(sys.executable); print(sys.version)" 2>&1
Write-Output "PY=$($py -join ' ')"
foreach ($f in @('orz.exe','orz-signer.exe','orz-acaf-provision.exe')) {
    Write-Output "PF_$f=$(Test-Path -LiteralPath (Join-Path 'C:\Program Files\orz' $f))"
}
'@
    Invoke-GuestJob -Name 'verify-relocation' -Body $bodyC

    # D) baseline checkpoint (pre-hardening)
    $vm = Get-VM -Name 'win-s4' -ErrorAction Stop
    $snapName = 'S4-BASE-INSTALLED'
    try {
        Checkpoint-VM -Name 'win-s4' -SnapshotName $snapName -Confirm:$false -ErrorAction Stop
        $lines.Add("===== SNAPSHOT $snapName OK =====")
    } catch {
        $lines.Add("SNAPSHOT_ERR: $($_.Exception.Message)")
    }

    # E) control-arm enforcement probe (pre-hardening baseline)
    $bodyD = @'
$env:Path = [Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [Environment]::GetEnvironmentVariable('Path','User')
& C:\s4\run\run_enforcement_probe.ps1 -Arm control -ResultPath C:\s4\results\enforcement-probe-control.json
exit $LASTEXITCODE
'@
    Invoke-GuestJob -Name 'probe-control' -Body $bodyD -TimeoutSeconds 300
}
catch {
    $lines.Add("ERROR: $($_.Exception.Message)")
}
$lines | Set-Content -LiteralPath $OutFile -Encoding utf8
