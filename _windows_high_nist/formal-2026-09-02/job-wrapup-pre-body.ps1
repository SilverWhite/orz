$env:Path = [Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [Environment]::GetEnvironmentVariable('Path','User')
$ErrorActionPreference = 'Continue'
$root = 'C:\workspace\wrapup'
New-Item -ItemType Directory -Path $root -Force | Out-Null
$summary = [ordered]@{
    ran_at = (Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ')
    tasks_removed = New-Object System.Collections.ArrayList
    procs_killed = New-Object System.Collections.ArrayList
    dirs_removed = New-Object System.Collections.ArrayList
    workspace_reset = $false
    hive_mounted_pre = $false
    hive_locked_pre = $false
    errors = New-Object System.Collections.ArrayList
}
# 1) stale scheduled tasks (legacy exact names; unique s4elevjob_* tasks are
#    removed by s4_vm_run_elev.ps1 itself and are not touched here)
$staleTasks = @('s4elevjob', 's4elevv10', 's4elevv11', 's4elevv12', 's4elevv13', 's4elevv14', 's4cred_direct', 's4cred')
foreach ($tn in $staleTasks) {
    & schtasks.exe /query /tn $tn 2>$null | Out-Null
    if ($LASTEXITCODE -eq 0) {
        & schtasks.exe /delete /tn $tn /f 2>$null | Out-Null
        [void]$summary.tasks_removed.Add("$tn removed=$($LASTEXITCODE -eq 0)")
    }
}
$csv = (& schtasks.exe /query /fo csv /nh 2>$null) -join "`n"
foreach ($line in ($csv -split "`n")) {
    if ($line -match '^"(s4(elevv\d+|cred[^"]*))"') {
        $tn = $Matches[1]
        if ($summary.tasks_removed -notcontains $tn) {
            & schtasks.exe /delete /tn $tn /f 2>$null | Out-Null
            [void]$summary.tasks_removed.Add("$tn removed=$($LASTEXITCODE -eq 0)")
        }
    }
}
# 2) stale repro/python processes (own ancestor chain is skipped)
$self = New-Object 'System.Collections.Generic.HashSet[int]'
[void]$self.Add([int]$PID)
try {
    $cur = Get-CimInstance Win32_Process -Filter "ProcessId=$PID" -ErrorAction Stop
    $guard = 0
    while ($cur -and $guard -lt 20) {
        $guard++
        $ppid = [int]$cur.ParentProcessId
        if ($ppid -le 0 -or $self.Contains($ppid)) { break }
        [void]$self.Add($ppid)
        $cur = Get-CimInstance Win32_Process -Filter "ProcessId=$ppid" -ErrorAction Stop
    }
}
catch { }
$staleMarkers = @('repro-v10', 'repro-v11', 'repro-v12', 'repro-v13', 'repro-v14', 'cred_probe', 's4elevv', 'mkprofile_formal')
$all = @(Get-CimInstance Win32_Process -ErrorAction SilentlyContinue |
    Where-Object { $_.Name -match 'python|powershell|cmd' -and $_.CommandLine -match 's4|elev|repro|cred|mkprofile' })
foreach ($p in $all) {
    if ($self.Contains([int]$p.ProcessId)) { continue }
    $hit = $false
    foreach ($m in $staleMarkers) {
        if ($p.CommandLine -match $m) { $hit = $true; break }
    }
    if (-not $hit) { continue }
    try {
        Stop-Process -Id ([int]$p.ProcessId) -Force -ErrorAction Stop
        [void]$summary.procs_killed.Add("$($p.ProcessId):$($p.Name)")
    }
    catch {
        [void]$summary.errors.Add("kill $($p.ProcessId) failed: $($_.Exception.Message)")
    }
}
# 3) old profile/config dirs and stale AppContainer package dirs
$oldDirs = @('C:\Users\AgentUser.DESKTOP-QMAPMFH', 'C:\WINDOWS\system32\config\AgentUser')
foreach ($d in $oldDirs) {
    if (Test-Path -LiteralPath $d) {
        try {
            Remove-Item -LiteralPath $d -Recurse -Force -ErrorAction Stop
            [void]$summary.dirs_removed.Add($d)
        }
        catch {
            [void]$summary.errors.Add("remove $d failed: $($_.Exception.Message)")
        }
    }
}
$pkgDirs = @(Get-ChildItem -LiteralPath 'C:\Users' -Directory -ErrorAction SilentlyContinue |
    ForEach-Object {
        $pkg = Join-Path $_.FullName 'AppData\Local\Packages'
        if (Test-Path -LiteralPath $pkg) {
            @(Get-ChildItem -LiteralPath $pkg -Directory -ErrorAction SilentlyContinue |
                Where-Object { $_.Name -match '^p2_native_run_' })
        }
    })
foreach ($d in $pkgDirs) {
    try {
        Remove-Item -LiteralPath $d.FullName -Recurse -Force -ErrorAction Stop
        [void]$summary.dirs_removed.Add($d.FullName)
    }
    catch {
        [void]$summary.errors.Add("remove $($d.FullName) failed: $($_.Exception.Message)")
    }
}
# 4) reset C:\workspace contents (evidence is already copied host-side)
try {
    @(Get-ChildItem -LiteralPath 'C:\workspace' -Force -ErrorAction Stop) |
        Remove-Item -Recurse -Force -ErrorAction Stop
    $summary.workspace_reset = $true
}
catch {
    [void]$summary.errors.Add("workspace reset failed: $($_.Exception.Message)")
}
New-Item -ItemType Directory -Path $root -Force | Out-Null
# 5) hive pre-state check: AgentUser NTUSER.DAT must not be mounted before
#    apply's HKCU freeze (a profile load without a matching unload locks the
#    file and makes the later reg load fail).
$u = [ADSI]"WinNT://$env:COMPUTERNAME/AgentUser"
$sidBytes = $u.objectSID[0]
$sid = [System.Security.Principal.SecurityIdentifier]::new($sidBytes, 0).Value
$hkuOut = (& reg.exe query HKU 2>&1) -join "`n"
$summary.hive_mounted_pre = $hkuOut.Contains($sid)
$hiveFile = 'C:\Users\AgentUser\NTUSER.DAT'
try {
    $fs = [System.IO.File]::Open($hiveFile, 'Open', 'ReadWrite', 'None')
    $fs.Close()
    $fs.Dispose()
    $summary.hive_locked_pre = $false
}
catch {
    $summary.hive_locked_pre = $true
    [void]$summary.errors.Add("NTUSER.DAT locked before apply: $($_.Exception.Message)")
}
Write-Output "HIVE_PRE mounted=$($summary.hive_mounted_pre) locked=$($summary.hive_locked_pre)"
$summary | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $root 'wrapup-pre-clean.json') -Encoding utf8
Write-Output 'WRAPUP_PRE_DONE'
exit 0
