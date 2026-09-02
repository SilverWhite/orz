<#
    Graceful shutdown of win-s4 VM with state report.
    Usage: powershell -NoProfile -ExecutionPolicy Bypass -File scripts\s4_vm_shutdown.ps1
    ASCII only.
#>
$ErrorActionPreference = 'Stop'
$vmName = 'win-s4'
$outDir = 'D:\CLI\_windows_high_nist'
$stamp = Get-Date -Format 'yyyyMMdd_HHmmss'
$resultFile = Join-Path $outDir ("vm-shutdown-result_{0}.txt" -f $stamp)

$lines = New-Object System.Collections.Generic.List[string]
function Write-Both([string]$msg) {
    $lines.Add($msg)
    Write-Output $msg
}

try {
    $vm = Get-VM -Name $vmName -ErrorAction Stop
    Write-Both ("STATE_BEFORE=" + $vm.State)
    Write-Both ("STATUS=" + $vm.Status)
    Write-Both ("INTEGRATION=" + $vm.IntegrationServicesState)
}
catch {
    Write-Both ("GETVM_ERROR=" + $_.Exception.Message)
    $lines | Out-File -Encoding utf8 $resultFile
    Write-Output ("resultFile=" + $resultFile)
    exit 1
}

$deadline = (Get-Date).AddMinutes(3)
$done = $false
if ($vm.State -eq 'Running') {
    Write-Both "ACTION=Stop-VM graceful"
    Stop-VM -Name $vmName -ErrorAction Stop
    while ((Get-Date) -lt $deadline) {
        Start-Sleep -Seconds 5
        $cur = Get-VM -Name $vmName -ErrorAction SilentlyContinue
        if ($null -ne $cur -and ($cur.State -eq 'Off' -or $cur.State -eq 'Saved')) {
            $done = $true
            Write-Both ("STATE_AFTER=" + $cur.State)
            break
        }
        if ($null -ne $cur) { Write-Both ("WAIT state=" + $cur.State) }
    }
    if (-not $done) {
        Write-Both 'RESULT=STILL_RUNNING_AFTER_GRACEFUL_TIMEOUT'
    }
    else {
        Write-Both 'RESULT=GRACEFUL_SHUTDOWN_OK'
    }
}
elseif ($vm.State -eq 'Off') {
    Write-Both 'ACTION=none (already off)'
    Write-Both 'RESULT=ALREADY_OFF'
}
elseif ($vm.State -eq 'Saved') {
    Write-Both 'ACTION=none (already saved)'
    Write-Both 'RESULT=ALREADY_SAVED'
}
else {
    Write-Both ("ACTION=none (state=" + $vm.State + ")")
    Write-Both 'RESULT=UNEXPECTED_STATE'
}

$lines | Out-File -Encoding utf8 $resultFile
Write-Output ("resultFile=" + $resultFile)
