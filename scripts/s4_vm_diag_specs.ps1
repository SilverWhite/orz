<#
    S4 VM restricted-token spec isolation (host-side elevated). ASCII only.
#>
[CmdletBinding()]
param(
    [string]$OutFile = 'D:\CLI\_windows_high_nist\vm-diag-specs-result.txt'
)

$ErrorActionPreference = 'Stop'
$lines = New-Object System.Collections.Generic.List[string]
$runner = 'D:\CLI\scripts\s4_vm_run_elev.ps1'

$body = @'
$env:PYTHONIOENCODING = 'utf-8'
$env:Path = [Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [Environment]::GetEnvironmentVariable('Path','User')
$env:PYTHONPATH = 'C:\s4'
$py = 'C:\Program Files\Python312\python.exe'
$codeLines = @(
    'import json, pathlib',
    'import assurance.windows_sandbox as ws',
    'base = pathlib.Path(r"C:\s4\diag-ws")',
    'base.mkdir(parents=True, exist_ok=True)',
    '(base / ".assurance-p2-disposable.json").write_text(json.dumps({"schema_version":"0.1.0-draft","purpose":"windows-native-sandbox-probe","allow_container_write_probe":True}), encoding="utf-8")',
    'orig = ws.build_restricted_token_spec',
    'def make_spec(variant):',
    '    def f(arm):',
    '        s = orig(arm)',
    '        if variant == "empty":',
    '            s = {"disable_sids": [], "deny_only_sids": [], "remove_privileges": [], "low_integrity": False, "appcontainer": False, "virtualization_allowed": True}',
    '        elif variant == "privs_only":',
    '            s["disable_sids"] = []; s["deny_only_sids"] = []',
    '        elif variant == "disable_only":',
    '            s["remove_privileges"] = []; s["virtualization_allowed"] = True',
    '        return s',
    '    return f',
    'for variant in ["empty", "privs_only", "disable_only", "all"]:',
    '    ws.build_restricted_token_spec = make_spec(variant)',
    '    obs = ws.run_windows_native_sandbox(["cmd.exe", "/c", "exit", "42"], arm="non-admin", workspace=base, timeout_seconds=30)',
    '    print("VARIANT", variant, "exit", obs.get("process", {}).get("exit_code"), "outcome", obs.get("outcome"))',
    '    for d in (obs.get("diagnostics") or []):',
    '        print("  DIAG:", d)',
    '    for k, v in (obs.get("checks") or {}).items():',
    '        print("  CHECK", k, v)'
)
$code = $codeLines -join [Environment]::NewLine
$code | Out-File -Encoding utf8 C:\s4\tools\diag-specs.py
& $py C:\s4\tools\diag-specs.py 2>&1 | ForEach-Object { Write-Output "PY: $_" }
'@

$bodyFile = 'D:\CLI\_windows_high_nist\job-diag-specs-body.ps1'
Set-Content -LiteralPath $bodyFile -Value $body -Encoding utf8
try {
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $runner `
        -JobBodyFile $bodyFile -OutFile $OutFile -TimeoutSeconds 400
    if ($LASTEXITCODE -ne 0) { throw "diag-specs job failed (exit $LASTEXITCODE)" }
}
catch {
    $lines.Add("ERROR: $($_.Exception.Message)")
    $lines | Set-Content -LiteralPath $OutFile -Encoding utf8
}
