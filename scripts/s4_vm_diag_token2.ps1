<#
    S4 VM restricted-token diagnostic after fix (host-side elevated):
    syncs windows_sandbox.py, then creates a non-admin restricted token and
    inspects the resulting token's group state.
    ASCII only.
#>
[CmdletBinding()]
param(
    [string]$OutFile = 'D:\CLI\_windows_high_nist\vm-diag-token2-result.txt'
)

$ErrorActionPreference = 'Stop'
$lines = New-Object System.Collections.Generic.List[string]
$runner = 'D:\CLI\scripts\s4_vm_run_elev.ps1'

try {
    $sec = ConvertTo-SecureString '123456' -AsPlainText -Force
    $cred = New-Object System.Management.Automation.PSCredential('HL', $sec)
    $sess = New-PSSession -VMName 'win-s4' -Credential $cred
    Copy-Item -LiteralPath 'D:\CLI\assurance\windows_sandbox.py' `
        -Destination 'C:\s4\assurance\windows_sandbox.py' -ToSession $sess -Force
    $srcHash = (Get-FileHash -LiteralPath 'D:\CLI\assurance\windows_sandbox.py' -Algorithm SHA256).Hash
    $dstHash = Invoke-Command -Session $sess -ScriptBlock {
        (Get-FileHash -LiteralPath 'C:\s4\assurance\windows_sandbox.py' -Algorithm SHA256).Hash
    }
    $lines.Add("SYNC=$($srcHash -eq $dstHash)")
    Remove-PSSession -Session $sess -ErrorAction SilentlyContinue
} catch {
    $lines.Add("SYNC_ERR: $($_.Exception.Message)")
}

$body = @'
$py = 'C:\Program Files\Python312\python.exe'
$env:PYTHONPATH = 'C:\s4'
$codeLines = @(
    'from assurance.windows_sandbox import _create_restricted_token, build_restricted_token_spec',
    'from assurance.windows_sandbox import _duplicate_token_primary, _token_has_enabled_group, _token_enabled_privileges',
    'import ctypes',
    'd = []',
    'spec = build_restricted_token_spec("non-admin")',
    't = _create_restricted_token(spec, diagnostics=d)',
    'print("TOKEN", t is not None)',
    'print("DIAGS", d)',
    'if t:',
    '    p = _duplicate_token_primary(t, diagnostics=d)',
    '    print("PRIMARY", p is not None)',
    '    if p:',
    '        print("ADMIN_ENABLED", _token_has_enabled_group(p, "S-1-5-32-544", diagnostics=d))',
    '        print("ENABLED_PRIVS", _token_enabled_privileges(p, diagnostics=d))',
    '        ctypes.windll.kernel32.CloseHandle(p)',
    '    ctypes.windll.kernel32.CloseHandle(t)'
)
$code = $codeLines -join [Environment]::NewLine
$code | Out-File -Encoding utf8 C:\s4\tools\diag-token2.py
& $py C:\s4\tools\diag-token2.py 2>&1 | ForEach-Object { Write-Output "PY: $_" }
'@

$bodyFile = 'D:\CLI\_windows_high_nist\job-diag-token2-body.ps1'
Set-Content -LiteralPath $bodyFile -Value $body -Encoding utf8
try {
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $runner `
        -JobBodyFile $bodyFile -OutFile $OutFile -TimeoutSeconds 300
    if ($LASTEXITCODE -ne 0) { throw "diag-token2 job failed (exit $LASTEXITCODE)" }
}
catch {
    $lines.Add("ERROR: $($_.Exception.Message)")
    $lines | Set-Content -LiteralPath $OutFile -Encoding utf8
}
