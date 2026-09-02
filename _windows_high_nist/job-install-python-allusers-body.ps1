$installer = 'C:\s4\tools\python-3.12.10-amd64.exe'
if (-not (Test-Path -LiteralPath $installer)) {
    Write-Output 'INSTALLER missing'
    exit 2
}
$installed = $false
for ($a = 1; $a -le 10; $a++) {
    try {
        $p = Start-Process -FilePath $installer `
            -ArgumentList @('/quiet', 'InstallAllUsers=1', 'PrependPath=1', 'Include_test=0', 'Include_launcher=1', 'AssociateFiles=0', 'Shortcuts=0', 'TargetDir=C:\Program Files\Python312') `
            -Wait -PassThru -ErrorAction Stop
        Write-Output "INSTALL attempt=$a exit=$($p.ExitCode)"
        if ($p.ExitCode -eq 0) { $installed = $true; break }
    } catch {
        Write-Output "INSTALL attempt=$a err=$($_.Exception.Message)"
        Start-Sleep -Seconds 3
    }
}
if (-not $installed) { exit 1 }
