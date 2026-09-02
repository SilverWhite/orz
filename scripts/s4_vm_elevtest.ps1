<#
    S4 VM scheduled-task elevation channel test (run elevated).
    ASCII only.
#>
[CmdletBinding()]
param(
    [string]$OutFile = 'D:\CLI\_windows_high_nist\vm-elevtest-result.txt'
)

$ErrorActionPreference = 'Stop'
$lines = New-Object System.Collections.Generic.List[string]

try {
    $sec = ConvertTo-SecureString '123456' -AsPlainText -Force
    $cred = New-Object System.Management.Automation.PSCredential('HL', $sec)
    $sess = New-PSSession -VMName 'win-s4' -Credential $cred
    Invoke-Command -Session $sess -ScriptBlock {
        New-Item -ItemType Directory -Path 'C:\s4\tools' -Force | Out-Null
    }
    Copy-Item -LiteralPath 'D:\CLI\scripts\s4_elev_probe.ps1' `
        -Destination 'C:\s4\tools\s4_elev_probe.ps1' -ToSession $sess -Force
    $guest = Invoke-Command -Session $sess -ScriptBlock {
        $out = New-Object System.Collections.Generic.List[string]
        $marker = 'C:\s4\tools\elev-test.txt'
        Remove-Item -LiteralPath $marker -Force -ErrorAction SilentlyContinue
        try {
            $taskName = 's4elevtest'
            Unregister-ScheduledTask -TaskName $taskName -Confirm:$false -ErrorAction SilentlyContinue
            $action = New-ScheduledTaskAction -Execute 'powershell.exe' `
                -Argument '-NoProfile -ExecutionPolicy Bypass -File C:\s4\tools\s4_elev_probe.ps1'
            $principal = New-ScheduledTaskPrincipal -UserId 'HL' -LogonType Password -RunLevel Highest
            $settings = New-ScheduledTaskSettingsSet -ExecutionTimeLimit (New-TimeSpan -Minutes 5) `
                -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries
            $task = New-ScheduledTask -Action $action -Principal $principal -Settings $settings
            Register-ScheduledTask -TaskName $taskName -InputObject $task `
                -User 'HL' -Password '123456' -Force | Out-Null
            Start-ScheduledTask -TaskName $taskName
            $deadline = (Get-Date).AddMinutes(2)
            while (-not (Test-Path -LiteralPath $marker) -and (Get-Date) -lt $deadline) {
                Start-Sleep -Seconds 2
            }
            Start-Sleep -Seconds 1
            $info = Get-ScheduledTaskInfo -TaskName $taskName
            $out.Add("TASK result=$($info.LastTaskResult) state=$((Get-ScheduledTask -TaskName $taskName).State)")
            if (Test-Path -LiteralPath $marker) {
                $out.Add("ELEV_MARKER content=$(Get-Content -LiteralPath $marker -Raw)")
            } else {
                $out.Add("ELEV_MARKER missing")
            }
            Unregister-ScheduledTask -TaskName $taskName -Confirm:$false
        } catch {
            $out.Add("TASK_ERR: $($_.Exception.Message)")
        }
        $out
    }
    foreach ($l in $guest) { $lines.Add($l) }
}
catch {
    $lines.Add("ERROR: $($_.Exception.Message)")
}
finally {
    if ($sess) {
        Remove-PSSession -Session $sess
    }
}
$lines | Set-Content -LiteralPath $OutFile -Encoding utf8
