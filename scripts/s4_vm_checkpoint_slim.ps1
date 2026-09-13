<#
    S4 VM checkpoint slim (host-side, ELEVATED).

    Purpose (2026-09-13 user decision): the win-s4 test VM only needs to stay
    usable for the high-nist policy runs; its internal data does NOT need to be
    preserved. So: stop the VM, delete every checkpoint (the chain merges into
    the base VHDX, keeping the CURRENT guest state), compact the base VHDX, and
    report before/after sizes.

    Run from an ADMIN PowerShell, e.g.:
      powershell -NoProfile -ExecutionPolicy Bypass -File D:\CLI\scripts\s4_vm_checkpoint_slim.ps1

    Options:
      -VMName <name>            default: win-s4
      -RecreateBaseCheckpoint   after slimming, create one fresh checkpoint
                                named S4-BASE-2026-09-13 (revert point for the
                                next batch). Default: off.
      -SkipCompact              skip Optimize-VHD (merge only).

    ASCII only on purpose (safe when launched elevated / piped).
#>
[CmdletBinding()]
param(
    [string]$VMName = 'win-s4',
    [switch]$RecreateBaseCheckpoint,
    [switch]$SkipCompact,
    [string]$EvidenceDir = 'D:\CLI\_windows_high_nist\evidence-vm-slim-20260913'
)

$ErrorActionPreference = 'Stop'
$stamp = Get-Date -Format 'yyyyMMdd_HHmmss'
$log = Join-Path $EvidenceDir ("vm-slim-{0}.log" -f $stamp)

function Say([string]$m) {
    $line = ('[{0}] {1}' -f (Get-Date -Format 'HH:mm:ss'), $m)
    Write-Host $line
    Add-Content -LiteralPath $log -Value $line -Encoding utf8
}

function DriveFree([string]$path) {
    $d = Split-Path -Qualifier (Resolve-Path -LiteralPath $path).Path
    $free = (Get-PSDrive -Name $d.TrimEnd(':')).Free
    return ('{0} free={1:N2} GB' -f $d, ($free / 1GB))
}

function ReportChain([string]$label) {
    Say "--- $label ---"
    Say (DriveFree 'D:\VMs')
    $vm = Get-VM -Name $VMName
    Say ("VM state = {0}" -f $vm.State)
    foreach ($snap in (Get-VMSnapshot -VMName $VMName | Sort-Object CreationTime)) {
        Say ("checkpoint: {0} | type={1} | created={2}" -f $snap.Name, $snap.SnapshotType, $snap.CreationTime)
    }
    foreach ($hd in (Get-VMHardDiskDrive -VMName $VMName)) {
        Say ("disk: {0}" -f $hd.Path)
        try {
            foreach ($item in @(Get-VHD -Path $hd.Path)) {
                Say ("  {0}  file={1:N2} GB  size={2:N2} GB  type={3}  parent={4}" -f `
                    (Split-Path $item.Path -Leaf), ($item.FileSize / 1GB), ($item.Size / 1GB), $item.VhdType, $item.ParentPath)
            }
        } catch {
            Say ("  Get-VHD failed: {0}" -f $_.Exception.Message)
        }
    }
    $files = Get-ChildItem -LiteralPath 'D:\VMs\' -Recurse -File -Include '*.vhdx','*.avhdx' -ErrorAction SilentlyContinue
    $sum = ($files | Measure-Object Length -Sum).Sum
    Say ("chain files: {0}  total={1:N2} GB" -f $files.Count, ($sum / 1GB))
}

# --- admin gate ------------------------------------------------------------
$principal = [Security.Principal.WindowsPrincipal]::new([Security.Principal.WindowsIdentity]::GetCurrent())
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    Write-Host 'ERROR: this script must run from an ELEVATED (Administrator) PowerShell.' -ForegroundColor Red
    exit 2
}

New-Item -ItemType Directory -Path $EvidenceDir -Force | Out-Null
Add-Content -LiteralPath $log -Value ("=== vm checkpoint slim {0} user={1} ===" -f $stamp, $env:USERNAME) -Encoding utf8

Import-Module Hyper-V -ErrorAction Stop

ReportChain 'BEFORE'

# --- stop the VM (internal data is explicitly not preserved) ---------------
$vm = Get-VM -Name $VMName
if ($vm.State -ne 'Off') {
    Say ("stopping VM (state={0}) - internal data is not preserved by design" -f $vm.State)
    Stop-VM -Name $VMName -Force -ErrorAction Stop
    $deadline = (Get-Date).AddMinutes(3)
    do {
        Start-Sleep -Seconds 2
        $state = (Get-VM -Name $VMName).State
    } while ($state -ne 'Off' -and (Get-Date) -lt $deadline)
    Say ("VM state after stop = {0}" -f $state)
}

# --- delete all checkpoints (merge chain into the base VHDX) ---------------
Say ("checkpoints to delete: {0}" -f @(Get-VMSnapshot -VMName $VMName).Count)
# Delete oldest-first and re-enumerate: removing the oldest checkpoint merges
# the whole chain below it into the base disk and removes its children, so any
# pre-computed list would go stale.
while (@(Get-VMSnapshot -VMName $VMName).Count -gt 0) {
    $s = Get-VMSnapshot -VMName $VMName | Sort-Object CreationTime | Select-Object -First 1
    Say ("deleting checkpoint: {0} (created {1})" -f $s.Name, $s.CreationTime)
    Remove-VMSnapshot -VMName $VMName -Name $s.Name -ErrorAction Stop
}
Say ('checkpoints remaining: ' + @(Get-VMSnapshot -VMName $VMName).Count)

# --- compact the base VHDX -------------------------------------------------
if (-not $SkipCompact) {
    foreach ($hd in (Get-VMHardDiskDrive -VMName $VMName)) {
        $vhd = Get-VHD -Path $hd.Path -ErrorAction SilentlyContinue
        if ($vhd -and $vhd.VhdType -eq 'Differencing') {
            # Explicit skip (2026-09-13 实测): Optimize-VHD does not support a
            # differencing leaf - it fails with 0x800700AA "resource in use".
            # The checkpoint merge above is what actually reclaims space; to
            # compact the BASE disk, remove the checkpoints first (base becomes
            # the active disk), compact, then recreate a checkpoint.
            Say ("skip compact: {0} is a differencing leaf (Optimize-VHD unsupported)" -f (Split-Path $hd.Path -Leaf))
            continue
        }
        Say ("compacting: {0}" -f $hd.Path)
        try {
            Optimize-VHD -Path $hd.Path -Mode Full -ErrorAction Stop
            Say '  Optimize-VHD Full OK'
        } catch {
            Say ("  Optimize-VHD Full failed: {0}" -f $_.Exception.Message)
            try {
                Optimize-VHD -Path $hd.Path -Mode Retrim -ErrorAction Stop
                Say '  Optimize-VHD Retrim OK'
            } catch {
                Say ("  Optimize-VHD Retrim failed: {0}" -f $_.Exception.Message)
            }
        }
    }
}

# --- optional fresh revert point ------------------------------------------
if ($RecreateBaseCheckpoint) {
    $name = 'S4-BASE-2026-09-13'
    if (@(Get-VMSnapshot -VMName $VMName | Where-Object { $_.Name -eq $name }).Count -eq 0) {
        Say ("creating fresh base checkpoint: {0}" -f $name)
        Checkpoint-VM -Name $VMName -SnapshotName $name -ErrorAction Stop
    } else {
        Say ("checkpoint already exists: {0}" -f $name)
    }
}

ReportChain 'AFTER'
Say ("log: {0}" -f $log)
Say 'done. paste this log back to the agent for ledger recording.'
