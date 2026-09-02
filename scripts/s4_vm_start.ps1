<#
    S4: start win-s4 VM (host-side elevated) and report state.
    ASCII only.
#>
[CmdletBinding()]
param(
    [string]$OutFile = 'D:\CLI\_windows_high_nist\vm-start-result.txt'
)

$ErrorActionPreference = 'Stop'
$lines = New-Object System.Collections.Generic.List[string]

try {
    $vm = Get-VM -Name 'win-s4' -ErrorAction Stop
    $lines.Add("VM=win-s4")
    $lines.Add("STATE_BEFORE=$($vm.State)")
    if ($vm.State -ne 'Running') {
        Start-VM -Name 'win-s4' -ErrorAction Stop
        $lines.Add("START_ISSUED=1")
    } else {
        $lines.Add("START_ISSUED=0")
    }
    $deadline = (Get-Date).AddSeconds(180)
    $state = $vm.State
    while ((Get-Date) -lt $deadline) {
        Start-Sleep -Seconds 3
        $vm = Get-VM -Name 'win-s4' -ErrorAction Stop
        $state = $vm.State
        if ($state -eq 'Running') { break }
    }
    $lines.Add("STATE_AFTER=$state")
    if ($state -eq 'Running') {
        $vmInfo = Get-VM -Name 'win-s4' -ErrorAction Stop
        $lines.Add("UPTIME_MIN=$([math]::Round(($(Get-Date) - $vmInfo.Uptime).TotalMinutes, 1))")
    }
}
catch {
    $lines.Add("ERROR: $($_.Exception.Message)")
}
$lines | Set-Content -LiteralPath $OutFile -Encoding utf8
