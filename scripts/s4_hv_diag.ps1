# Hyper-V hypervisor load-state diagnostic (host-side, read-only by default).
# Case: docs/cases/windows/ORZ-WIN-HV-001-hypervisor-load-diagnosis.md
# ASCII only (ORZ-WIN-PS-001 lesson 1: PS 5.1 misparses BOM-less non-ASCII).
#
# Usage:
#   powershell -NoProfile -ExecutionPolicy Bypass -File scripts/s4_hv_diag.ps1
#   powershell ... -File scripts/s4_hv_diag.ps1 -StartProbe   # also try Start-VM
#
# Answers one question in three layers:
#   L1 config  : is hypervisor launch configured and features enabled?
#   L2 runtime : is the hypervisor actually loaded THIS boot?
#   L3 evidence: what does the event log say (authoritative)?
#
# Key lesson: vmms/vmcompute RUNNING does NOT imply the hypervisor is loaded,
# and HypervisorPresent/VirtualizationFirmwareEnabled readings can be False on
# machines where virtualization fully works (firmware-report quirk). The event
# log + a functional VM-start probe are the only authoritative signals.

[CmdletBinding()]
param(
    [switch]$StartProbe,
    [string]$VmName = 'win-s4',
    [int]$EventDays = 3
)

$ErrorActionPreference = 'SilentlyContinue'

function Line([string]$t) { Write-Output ('--- ' + $t + ' ---') }

Line 'L1 CONFIG'
$bcd = bcdedit /enum '{current}' 2>&1 | Select-String 'hypervisorlaunchtype'
Write-Output ("bcdedit hypervisorlaunchtype : " + $(if ($bcd) { $bcd.ToString().Trim() } else { 'ENTRY MISSING' }))
foreach ($f in 'VirtualMachinePlatform', 'Microsoft-Hyper-V-All', 'HypervisorPlatform') {
    $feat = Get-WindowsOptionalFeature -Online -FeatureName $f
    Write-Output ("feature " + $f + " : " + $(if ($feat) { $feat.State } else { 'QUERY FAILED' }))
}

Line 'L2 RUNTIME (this boot)'
$cs = Get-CimInstance Win32_ComputerSystem
Write-Output ("HypervisorPresent            : " + $cs.HypervisorPresent + "  (False alone is NOT a BIOS verdict)")
$pf = Get-CimInstance Win32_Processor
Write-Output ("VirtualizationFirmwareEnabled: " + $pf.VirtualizationFirmwareEnabled + "  (known false-negative quirk on some boards; see case)")
$svc = Get-Service vmms, vmcompute
$svc | ForEach-Object { Write-Output ("service " + $_.Name + " : " + $_.Status) }
Write-Output ('NOTE: management services running != hypervisor kernel loaded (two layers).')
Write-Output ("LastBootUpTime               : " + (Get-CimInstance Win32_OperatingSystem).LastBootUpTime)
Write-Output ('       fast-startup/hiberboot resumes are the usual cause of a boot without hypervisor.')

Line 'L2 PHYSICAL vs GUEST (firmware identity)'
$bios = Get-CimInstance Win32_BIOS
$base = Get-CimInstance Win32_BaseBoard
Write-Output ("BIOS vendor/product : " + $bios.Manufacturer + ' / ' + $bios.SMBIOSBIOSVersion)
Write-Output ("Board vendor/product: " + $base.Manufacturer + ' / ' + $base.Product)

Line 'L2 FUNCTIONAL PROBE (decisive)'
$vm = Get-VM -Name $VmName
if ($vm) {
    Write-Output ("VM " + $VmName + " state before : " + $vm.State)
    if ($StartProbe) {
        Start-VM -Name $VmName -ErrorAction SilentlyContinue | Out-Null
        Start-Sleep -Seconds 12
        $vm = Get-VM -Name $VmName
        Write-Output ("VM " + $VmName + " state after 12s : " + $vm.State)
        Write-Output ('Still Off after a silent Start-VM => hypervisor not loaded; read L3 event log.')
    } else {
        Write-Output ('(pass -StartProbe to attempt an actual start; it is the decisive test)')
    }
} else {
    Write-Output ('VM ' + $VmName + ' not found; enumerate with Get-VM.')
}

Line 'L3 EVENT LOG (authoritative, Microsoft-Windows-Hyper-V-VMMS-Admin)'
$since = (Get-Date).AddDays(-1 * $EventDays)
$ev = Get-WinEvent -LogName 'Microsoft-Windows-Hyper-V-VMMS-Admin' -MaxEvents 400 |
    Where-Object { $_.TimeCreated -gt $since -and $_.Id -in 15130, 20148, 33540 }
if ($ev) {
    $ev | Select-Object -First 6 TimeCreated, Id | Format-Table -AutoSize | Out-String -Width 120 | Write-Output
    $first = $ev | Where-Object { $_.Id -eq 20148 } | Select-Object -First 1
    if ($first) {
        Write-Output ('Event 20148 message (truncated):')
        Write-Output (($first.Message -replace "`r`n", ' ').Substring(0, [Math]::Min(300, $first.Message.Length)))
    }
} else {
    Write-Output ('No 15130/20148/33540 events in the last ' + $EventDays + ' day(s).')
}
Write-Output ('Interpretation: 20148 "hypervisor not running" = real fault (this boot).')
Write-Output ('Absent 20148 + VM starts fine = readings quirk only; proceed with work.')

Line 'VERDICT HELPER'
if ($ev | Where-Object { $_.Id -eq 20148 }) {
    Write-Output 'REAL FAULT: hypervisor not loaded this boot. Fix ladder: normal restart -> cold power-on -> BIOS re-save VT-x.'
} elseif ($vm -and $vm.State -eq 'Running') {
    Write-Output 'HEALTHY (or quirk-only): VM runs despite False readings; treat API readings as unreliable on this board.'
} else {
    Write-Output 'INCONCLUSIVE: re-run with -StartProbe, then compare against L3.'
}
