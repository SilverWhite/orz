$env:Path = [Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [Environment]::GetEnvironmentVariable('Path','User')
$env:PYTHONPATH = 'C:\s4'
$py = 'C:\Program Files\Python312\python.exe'
Write-Output '--- AppLocker policy present? ---'
$srp = reg query 'HKLM\SOFTWARE\Policies\Microsoft\Windows\SrpV2' 2>&1
Write-Output "SRP_QUERY exit=$LASTEXITCODE : $($srp -join ' ')".Trim()
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
    '        elif variant == "disable_only":',
    '            s["remove_privileges"] = []; s["virtualization_allowed"] = True',
    '        return s',
    '    return f',
    'def runit(label):',
    '    ws.build_restricted_token_spec = make_spec("disable_only")',
    '    obs = ws.run_windows_native_sandbox(["cmd.exe", "/c", "exit", "42"], arm="non-admin", workspace=base, timeout_seconds=30)',
    '    print(label, "exit", obs.get("process", {}).get("exit_code"), "outcome", obs.get("outcome"))',
    'runit("BEFORE_APPLOCKER_OFF")',
    'import subprocess',
    'r = subprocess.run(["reg.exe", "delete", r"HKLM\SOFTWARE\Policies\Microsoft\Windows\SrpV2", "/f"], capture_output=True)',
    'print("REG_DELETE", r.returncode, (r.stdout + r.stderr).decode(errors="replace"))',
    'runit("AFTER_APPLOCKER_OFF")'
)
$code = $codeLines -join [Environment]::NewLine
$code | Out-File -Encoding utf8 C:\s4\tools\diag-applocker.py
& $py C:\s4\tools\diag-applocker.py 2>&1 | ForEach-Object { Write-Output "PY: $_" }
