<#
    S4 VM PowerShell Direct session diagnostic (host-side elevated).
    ASCII only.
#>
[CmdletBinding()]
param(
    [string]$OutFile = 'D:\CLI\_windows_high_nist\vm-diag-session-result.txt'
)

$ErrorActionPreference = 'Stop'
$lines = New-Object System.Collections.Generic.List[string]

try {
    $sec = ConvertTo-SecureString '123456' -AsPlainText -Force
    $cred = New-Object System.Management.Automation.PSCredential('HL', $sec)
    $lines.Add('TRY New-PSSession -VMName...')
    $sess = New-PSSession -VMName 'win-s4' -Credential $cred -ErrorAction Stop
    $lines.Add("SESSION_OK=$($sess.Id)")
    $res = Invoke-Command -Session $sess -ScriptBlock {
        "GUEST_OK user=$([System.Security.Principal.WindowsIdentity]::GetCurrent().Name) applocker=$(Get-AppLockerPolicy -Effective -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Policy | Out-String | Select-Object -First 1)"
    } -ErrorAction Stop
    $lines.Add(($res | Out-String).Trim())
    Remove-PSSession -Session $sess -ErrorAction SilentlyContinue
    $lines.Add('SESSION_CLOSE_OK')
}
catch {
    $lines.Add("ERROR: $($_.Exception.Message)")
    $lines.Add("STACK: $($_.ScriptStackTrace)")
}
$lines | Set-Content -LiteralPath $OutFile -Encoding utf8
