<#
    S4 VM UAC/task policy inspector (run elevated). ASCII only.
#>
[CmdletBinding()]
param(
    [string]$OutFile = 'D:\CLI\_windows_high_nist\vm-uac-result.txt'
)

$ErrorActionPreference = 'Stop'
$lines = New-Object System.Collections.Generic.List[string]

try {
    $sec = ConvertTo-SecureString '123456' -AsPlainText -Force
    $cred = New-Object System.Management.Automation.PSCredential('HL', $sec)
    $sess = New-PSSession -VMName 'win-s4' -Credential $cred
    $guest = Invoke-Command -Session $sess -ScriptBlock {
        $out = New-Object System.Collections.Generic.List[string]
        $base = 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\System'
        foreach ($name in @('EnableLUA', 'ConsentPromptBehaviorAdmin', 'PromptOnSecureDesktop', 'LocalAccountTokenFilterPolicy', 'FilterAdministratorToken')) {
            $v = Get-ItemProperty -Path $base -Name $name -ErrorAction SilentlyContinue
            if ($v) {
                $out.Add("$name=$($v.$name)")
            } else {
                $out.Add("$name=<absent>")
            }
        }
        $taskName = 's4elevxml'
        Unregister-ScheduledTask -TaskName $taskName -Confirm:$false -ErrorAction SilentlyContinue
        $action = New-ScheduledTaskAction -Execute 'powershell.exe' `
            -Argument '-NoProfile -ExecutionPolicy Bypass -File C:\s4\tools\s4_elev_probe.ps1'
        $principal = New-ScheduledTaskPrincipal -UserId 'HL' -LogonType Password -RunLevel Highest
        $settings = New-ScheduledTaskSettingsSet -ExecutionTimeLimit (New-TimeSpan -Minutes 5) `
            -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries
        $task = New-ScheduledTask -Action $action -Principal $principal -Settings $settings
        Register-ScheduledTask -TaskName $taskName -InputObject $task -User 'HL' -Password '123456' -Force | Out-Null
        $xml = Export-ScheduledTask -TaskName $taskName
        $out.Add('--- TASK XML ---')
        foreach ($l in ($xml -split "`r?`n")) {
            if ($l -match 'RunLevel|UserId|LogonType|Principal') { $out.Add($l.Trim()) }
        }
        Unregister-ScheduledTask -TaskName $taskName -Confirm:$false
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
