$installer = 'C:\s4\tools\python-3.12.10-amd64.exe'
$u = Start-Process -FilePath $installer -ArgumentList @('/uninstall','/quiet','InstallAllUsers=0') -Wait -PassThru -ErrorAction SilentlyContinue
Write-Output "UNINSTALL_PERUSER exit=$($u.ExitCode)"
Start-Sleep -Seconds 3
$legacy = 'C:\Users\HL\AppData\Local\Programs\Python'
if (Test-Path -LiteralPath $legacy) {
    Remove-Item -LiteralPath $legacy -Recurse -Force -ErrorAction SilentlyContinue
    Write-Output "LEGACY_REMOVED=$(-not (Test-Path -LiteralPath $legacy))"
}
$log = 'C:\s4\tools\py-allusers-install.log'
Remove-Item -LiteralPath $log -Force -ErrorAction SilentlyContinue
$installed = $false
for ($a = 1; $a -le 5; $a++) {
    try {
        $p = Start-Process -FilePath $installer `
            -ArgumentList @('/quiet','InstallAllUsers=1','PrependPath=1','Include_test=0','Include_launcher=1','AssociateFiles=0','Shortcuts=0','TargetDir=C:\Program Files\Python312','/log',$log) `
            -Wait -PassThru -ErrorAction Stop
        Write-Output "INSTALL attempt=$a exit=$($p.ExitCode)"
        if ($p.ExitCode -eq 0) { $installed = $true; break }
    } catch {
        Write-Output "INSTALL attempt=$a err=$($_.Exception.Message)"
        Start-Sleep -Seconds 3
    }
}
if (-not $installed) {
    if (Test-Path -LiteralPath $log) {
        Write-Output '--- install log tail ---'
        Get-Content -LiteralPath $log -Tail 40 | ForEach-Object { Write-Output $_ }
    }
    exit 1
}
$env:Path = [Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [Environment]::GetEnvironmentVariable('Path','User')
$py = & python -c "import sys; print(sys.executable); print(sys.version)" 2>&1
Write-Output "PY=$($py -join ' ')"
$py2 = & py -3 -c "import sys; print(sys.executable)" 2>&1
Write-Output "PY3=$($py2 -join ' ')"
