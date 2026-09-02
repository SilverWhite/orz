<#
    S4 VM advapi32 StringSidToSidW availability check (host-side elevated).
    ASCII only.
#>
[CmdletBinding()]
param(
    [string]$OutFile = 'D:\CLI\_windows_high_nist\vm-diag-sid-result.txt'
)

$ErrorActionPreference = 'Stop'
$lines = New-Object System.Collections.Generic.List[string]
$runner = 'D:\CLI\scripts\s4_vm_run_elev.ps1'

$body = @'
$py = 'C:\Program Files\Python312\python.exe'
& $py -c "import ctypes; a=ctypes.WinDLL('advapi32', use_last_error=True); print('StringSidToSidW', hasattr(a,'StringSidToSidW')); print('StringSidToSidA', hasattr(a,'StringSidToSidA')); print('StringSidToSid', hasattr(a,'StringSidToSid')); print('ConvertStringSidToSidW', hasattr(a,'ConvertStringSidToSidW'))" 2>&1 | ForEach-Object { Write-Output "PY: $_" }
$sig = Get-Command -Name 'StringSidToSidW' -ErrorAction SilentlyContinue
Write-Output "DLL_EXPORT_CHECK=$(if ($sig) { 'cmdlet' } else { 'n/a' })"
'@

$bodyFile = 'D:\CLI\_windows_high_nist\job-diag-sid-body.ps1'
Set-Content -LiteralPath $bodyFile -Value $body -Encoding utf8
try {
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $runner `
        -JobBodyFile $bodyFile -OutFile $OutFile -TimeoutSeconds 300
    if ($LASTEXITCODE -ne 0) { throw "sid diag job failed (exit $LASTEXITCODE)" }
}
catch {
    $lines.Add("ERROR: $($_.Exception.Message)")
    $lines | Set-Content -LiteralPath $OutFile -Encoding utf8
}
