<#
    S4 VM current scheduled-task token diagnostic (host-side elevated).
    ASCII only.
#>
[CmdletBinding()]
param(
    [string]$OutFile = 'D:\CLI\_windows_high_nist\vm-diag-tasktoken-result.txt'
)

$ErrorActionPreference = 'Stop'
$lines = New-Object System.Collections.Generic.List[string]
$runner = 'D:\CLI\scripts\s4_vm_run_elev.ps1'

$body = @'
Write-Output '--- whoami /groups (admin + integrity) ---'
whoami /groups | Select-String -Pattern 'S-1-5-32-544|Mandatory' | ForEach-Object { Write-Output $_.Line }
Write-Output '--- whoami /priv ---'
whoami /priv
Write-Output "PRIV_EXIT=$LASTEXITCODE"
Write-Output '--- net session ---'
net session 2>&1 | Select-Object -First 3 | ForEach-Object { Write-Output "NET: $_" }
Write-Output "NET_EXIT=$LASTEXITCODE"
$id = [System.Security.Principal.WindowsIdentity]::GetCurrent()
$p = [System.Security.Principal.WindowsPrincipal]::new($id)
Write-Output "ISADMIN=$($p.IsInRole([System.Security.Principal.WindowsBuiltInRole]::Administrator))"
'@

$bodyFile = 'D:\CLI\_windows_high_nist\job-diag-tasktoken-body.ps1'
Set-Content -LiteralPath $bodyFile -Value $body -Encoding utf8
try {
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $runner `
        -JobBodyFile $bodyFile -OutFile $OutFile -TimeoutSeconds 300
    if ($LASTEXITCODE -ne 0) { throw "diag-tasktoken job failed (exit $LASTEXITCODE)" }
}
catch {
    $lines.Add("ERROR: $($_.Exception.Message)")
    $lines | Set-Content -LiteralPath $OutFile -Encoding utf8
}
