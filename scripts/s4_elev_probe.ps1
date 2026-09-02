<#
    S4 guest-side elevation probe: writes token facts to a marker file.
    ASCII only.
#>
[CmdletBinding()]
param(
    [string]$Marker = 'C:\s4\tools\elev-test.txt'
)

$id = [System.Security.Principal.WindowsIdentity]::GetCurrent()
$p = [System.Security.Principal.WindowsPrincipal]::new($id)
$highIL = ($id.Groups | Where-Object { $_.Value -eq 'S-1-16-12288' }) -ne $null
$mediumIL = ($id.Groups | Where-Object { $_.Value -eq 'S-1-16-8192' }) -ne $null
$groups = (& whoami /groups) -join "`n"
"user=$($id.Name) admin=$($p.IsInRole([System.Security.Principal.WindowsBuiltInRole]::Administrator)) highIL=$highIL mediumIL=$mediumIL`n$groups" |
    Set-Content -LiteralPath $Marker -Encoding ascii
