<#
    S4 VM cleanup helper (run elevated): kills orphaned PowerShell Direct
    socket hosts older than a cutoff and removes the corrupt python
    installer so a fresh download can proceed.
    ASCII only.
#>
[CmdletBinding()]
param(
    [string]$OutFile = 'D:\CLI\_windows_high_nist\vm-cleanup-result.txt'
)

$ErrorActionPreference = 'Stop'
$lines = New-Object System.Collections.Generic.List[string]

try {
    $sec = ConvertTo-SecureString '123456' -AsPlainText -Force
    $cred = New-Object System.Management.Automation.PSCredential('HL', $sec)
    $sess = New-PSSession -VMName 'win-s4' -Credential $cred
    $guest = Invoke-Command -Session $sess -ScriptBlock {
        $out = New-Object System.Collections.Generic.List[string]
        $cutoff = (Get-Date).AddMinutes(-5)
        $orphans = Get-CimInstance Win32_Process | Where-Object {
            $_.Name -eq 'powershell.exe' -and
            $_.CommandLine -match 'V2SocketServerMode' -and
            $_.CreationDate -lt $cutoff
        }
        foreach ($p in $orphans) {
            try {
                Stop-Process -Id $p.ProcessId -Force -ErrorAction Stop
                $out.Add("KILLED pid=$($p.ProcessId) start=$($p.CreationDate)")
            } catch {
                $out.Add("KILL_ERR pid=$($p.ProcessId): $($_.Exception.Message)")
            }
        }
        if (-not $orphans) { $out.Add('NO_ORPHANS') }
        $inst = 'C:\s4\tools\python-3.12.10-amd64.exe'
        if (Test-Path -LiteralPath $inst) {
            try {
                Remove-Item -LiteralPath $inst -Force -ErrorAction Stop
                $out.Add('INSTALLER removed')
            } catch {
                $out.Add("INSTALLER_REMOVE_ERR: $($_.Exception.Message)")
            }
        } else {
            $out.Add('INSTALLER absent')
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
