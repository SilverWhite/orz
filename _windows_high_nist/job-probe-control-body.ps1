$env:Path = [Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [Environment]::GetEnvironmentVariable('Path','User')
New-Item -ItemType Directory -Path 'C:\s4\results' -Force | Out-Null
& C:\s4\_windows_high_nist\run\run_enforcement_probe.ps1 -Arm control -ResultPath C:\s4\results\enforcement-probe-control.json
exit $LASTEXITCODE
