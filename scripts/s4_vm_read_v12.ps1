<#
    S4: read the partial v12 job log + VM process/task state (host elevated).
    ASCII only.
#>
[CmdletBinding()]
param(
    [string]$OutFile = 'D:\CLI\_windows_high_nist\vm-read-v12-result.txt'
)

$ErrorActionPreference = 'Stop'
$lines = New-Object System.Collections.Generic.List[string]
try {
    $sec = ConvertTo-SecureString '123456' -AsPlainText -Force
    $cred = New-Object System.Management.Automation.PSCredential('HL', $sec)
    $deadline = (Get-Date).AddSeconds(240)
    $up = $false
    while ((Get-Date) -lt $deadline) {
        try {
            $t = New-PSSession -VMName 'win-s4' -Credential $cred -ErrorAction Stop
            Remove-PSSession -Session $t -ErrorAction SilentlyContinue
            $up = $true
            break
        } catch {
            Start-Sleep -Seconds 10
        }
    }
    if (-not $up) {
        throw 'win-s4 not reachable within 240s'
    }
    $sess = New-PSSession -VMName 'win-s4' -Credential $cred
    $log = Invoke-Command -Session $sess -ScriptBlock {
        if (Test-Path -LiteralPath 'C:\s4\tools\v12-job.log') {
            Get-Content -LiteralPath 'C:\s4\tools\v12-job.log'
        } else {
            'NO_LOG'
        }
    }
    foreach ($l in $log) { $lines.Add("LOG: $l") }
    $proc = Invoke-Command -Session $sess -ScriptBlock {
        tasklist.exe /fi "IMAGENAME eq python.exe" /fo csv 2>&1
        tasklist.exe /fi "IMAGENAME eq cmd.exe" /fo csv 2>&1
    }
    foreach ($l in $proc) { $lines.Add("PROC: $l") }
    $tasks = Invoke-Command -Session $sess -ScriptBlock {
        & schtasks.exe /query /tn s4elevv12 /v /fo list 2>&1
    }
    foreach ($l in $tasks) { $lines.Add("TASK: $l") }
    Remove-PSSession -Session $sess -ErrorAction SilentlyContinue
} catch {
    $lines.Add("ERROR: $($_.Exception.Message)")
}
$lines | Set-Content -LiteralPath $OutFile -Encoding utf8
