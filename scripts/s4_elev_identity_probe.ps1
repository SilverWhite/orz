<#
    S4: print the identity of the current (elevated) process to a file.
    ASCII only.
#>
[CmdletBinding()]
param(
    [string]$OutFile = 'D:\CLI\_windows_high_nist\elev-id-check.txt'
)
$id = [System.Security.Principal.WindowsIdentity]::GetCurrent()
$p = [System.Security.Principal.WindowsPrincipal]::new($id)
$lines = @(
    "USER=$($id.Name)",
    ("ISADMIN=" + $p.IsInRole([System.Security.Principal.WindowsBuiltInRole]::Administrator)),
    "SESSION=$([System.Diagnostics.Process]::GetCurrentProcess().SessionId)",
    "ELEVATED=$([Environment]::UserInteractive)"
)
$groups = $id.Groups | ForEach-Object {
    try {
        $n = $_.Translate([System.Security.Principal.NTAccount]).Value
    }
    catch {
        $n = $_.Value
    }
    "$n=$($_.Value)"
}
$lines += "GROUPS=" + ($groups -join '|')
$il = $id.Groups | Where-Object { $_.Value -match '^S-1-16-' } | Select-Object -First 1
$lines += "INTEGRITY=$($il.Value)"
$lines | Set-Content -LiteralPath $OutFile -Encoding utf8
