<#
    S4 VM fix machine-wide python install, part 2 (host-side elevated).
    Uninstalls the mis-targeted python, removes C:\Program, cleans machine
    PATH, reinstalls to C:\Program Files\Python312 with a quoted TargetDir,
    and verifies resolution.
    ASCII only.
#>
[CmdletBinding()]
param(
    [string]$OutFile = 'D:\CLI\_windows_high_nist\vm-python-fix2-result.txt'
)

$ErrorActionPreference = 'Stop'
$lines = New-Object System.Collections.Generic.List[string]
$runner = 'D:\CLI\scripts\s4_vm_run_elev.ps1'

$body = @'
$installer = 'C:\s4\tools\python-3.12.10-amd64.exe'
$path = [Environment]::GetEnvironmentVariable('Path','Machine')
$path | Out-File -Encoding utf8 'C:\s4\tools\machine-path-backup.txt'
$u = Start-Process -FilePath $installer -ArgumentList @('/uninstall','/quiet','InstallAllUsers=1') -Wait -PassThru -ErrorAction SilentlyContinue
Write-Output "UNINSTALL_MACHINE exit=$($u.ExitCode)"
Start-Sleep -Seconds 3
if (Test-Path -LiteralPath 'C:\Program') {
    Remove-Item -LiteralPath 'C:\Program' -Recurse -Force -ErrorAction SilentlyContinue
    Write-Output "C:\Program removed=$(-not (Test-Path -LiteralPath 'C:\Program'))"
}
$entries = $path -split ';' | Where-Object {
    $_ -and $_ -notmatch '^C:\\Program(\\Scripts)?$' -and $_ -notmatch '^C:\\Program Files\\Python312'
}
$newPath = $entries -join ';'
[Environment]::SetEnvironmentVariable('Path', $newPath, 'Machine')
Write-Output "PATH_CLEANED=$newPath"
$log = 'C:\s4\tools\py-allusers-install2.log'
Remove-Item -LiteralPath $log -Force -ErrorAction SilentlyContinue
$installed = $false
for ($a = 1; $a -le 5; $a++) {
    try {
        $p = Start-Process -FilePath $installer `
            -ArgumentList @('/quiet','InstallAllUsers=1','PrependPath=1','Include_test=0','Include_launcher=1','AssociateFiles=0','Shortcuts=0','TargetDir="C:\Program Files\Python312"','/log',$log) `
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
Write-Output "FINAL_MACHINE_PATH=$([Environment]::GetEnvironmentVariable('Path','Machine'))"
$py = & python -c "import sys; print(sys.executable); print(sys.version)" 2>&1
Write-Output "PY=$($py -join ' ')"
Write-Output "PF_EXISTS=$(Test-Path -LiteralPath 'C:\Program Files\Python312\python.exe')"
'@

$bodyFile = 'D:\CLI\_windows_high_nist\job-fix-python2-body.ps1'
Set-Content -LiteralPath $bodyFile -Value $body -Encoding utf8
try {
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $runner `
        -JobBodyFile $bodyFile -OutFile $OutFile -TimeoutSeconds 1500
    if ($LASTEXITCODE -ne 0) { throw "fix job failed (exit $LASTEXITCODE)" }
}
catch {
    $lines.Add("ERROR: $($_.Exception.Message)")
    $lines | Set-Content -LiteralPath $OutFile -Encoding utf8
}
