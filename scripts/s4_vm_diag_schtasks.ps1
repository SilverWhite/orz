<#
    S4 VM post-hardening CIM vs schtasks diagnostic (host-side elevated).
    ASCII only.
#>
[CmdletBinding()]
param(
    [string]$OutFile = 'D:\CLI\_windows_high_nist\vm-diag-schtasks-result.txt'
)

$ErrorActionPreference = 'Stop'
$lines = New-Object System.Collections.Generic.List[string]

try {
    $sec = ConvertTo-SecureString '123456' -AsPlainText -Force
    $cred = New-Object System.Management.Automation.PSCredential('HL', $sec)
    $sess = New-PSSession -VMName 'win-s4' -Credential $cred
    $guest = Invoke-Command -Session $sess -ScriptBlock {
        $out = New-Object System.Collections.Generic.List[string]
        try {
            $c = Get-CimInstance Win32_ComputerSystem -ErrorAction Stop
            $out.Add("CIM_WIN32_OK name=$($c.Name)")
        } catch { $out.Add("CIM_WIN32_ERR=$($_.Exception.Message)") }
        try {
            $t = Get-ScheduledTask -TaskName 's4elevjob' -ErrorAction Stop
            $out.Add("GET_SCHEDTASK_OK state=$($t.State)")
        } catch { $out.Add("GET_SCHEDTASK_ERR=$($_.Exception.Message)") }
        $sq = & schtasks.exe /query /tn s4elevjob 2>&1
        $out.Add("SCHTASKS_QUERY exit=$LASTEXITCODE : $($sq -join ' ')".Trim())
        $out
    }
    foreach ($l in $guest) { $lines.Add($l) }
    Remove-PSSession -Session $sess -ErrorAction SilentlyContinue
}
catch {
    $lines.Add("ERROR: $($_.Exception.Message)")
}
$lines | Set-Content -LiteralPath $OutFile -Encoding utf8
