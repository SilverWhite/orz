<#
    S4 VM process/log inspector (run elevated). ASCII only.
#>
[CmdletBinding()]
param(
    [string]$OutFile = 'D:\CLI\_windows_high_nist\vm-proc-result.txt'
)

$ErrorActionPreference = 'Stop'
$lines = New-Object System.Collections.Generic.List[string]

try {
    $sec = ConvertTo-SecureString '123456' -AsPlainText -Force
    $cred = New-Object System.Management.Automation.PSCredential('HL', $sec)
    $sess = New-PSSession -VMName 'win-s4' -Credential $cred
    $guest = Invoke-Command -Session $sess -ScriptBlock {
        $out = New-Object System.Collections.Generic.List[string]
        $procs = Get-CimInstance Win32_Process | Sort-Object CreationDate |
            Select-Object ProcessId, ParentProcessId, CreationDate, Name, CommandLine
        foreach ($p in $procs) {
            if ($p.Name -match 'powershell|pwsh|cmd|curl|python|msiexec|wusa|wsmprovhost|conhost|svchost|wmiprvse') {
                $out.Add("PROC pid=$($p.ProcessId) ppid=$($p.ParentProcessId) name=$($p.Name) start=$($p.CreationDate)")
                if ($p.CommandLine) { $out.Add("  CMD: $($p.CommandLine)") }
            }
        }
        if (-not $procs) { $out.Add('PROC none') }
        $log = 'C:\s4\tools\py-install.log'
        if (Test-Path -LiteralPath $log) {
            $out.Add('--- py-install.log ---')
            foreach ($l in (Get-Content -LiteralPath $log)) { $out.Add($l) }
        } else {
            $out.Add('LOG missing')
        }
        $inst = 'C:\s4\tools\python-3.12.10-amd64.exe'
        if (Test-Path -LiteralPath $inst) {
            $out.Add("INSTALLER size=$((Get-Item -LiteralPath $inst).Length)")
            $item = Get-Item -LiteralPath $inst
            $out.Add("ATTRIB readOnly=$($item.IsReadOnly) hidden=$($item.Attributes -band [IO.FileAttributes]::Hidden)")
            try {
                $fs = [System.IO.File]::Open($inst, [System.IO.FileMode]::Open, [System.IO.FileAccess]::ReadWrite, [System.IO.FileShare]::None)
                $fs.Close()
                $out.Add("LOCKTEST open-exclusive OK (not locked)")
            } catch {
                $out.Add("LOCKTEST locked: $($_.Exception.Message)")
            }
            try {
                [System.IO.File]::Delete($inst)
                $out.Add("DELETE net-ok")
            } catch {
                $out.Add("DELETE err: $($_.Exception.Message)")
            }
        } else {
            $out.Add('INSTALLER missing')
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
