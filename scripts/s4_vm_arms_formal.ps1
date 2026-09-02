<#
    S4 formal three-arm sequence driver (host-side elevated).

    Stage restore  - stop VM if running, restore baseline checkpoint
                     S4-BASE-INSTALLED, boot, wait for PowerShell Direct.
    Stage sync     - copy current harness files into the VM, hash-verify.
    Stage setup    - re-provision AgentUser (password, SeBatchLogonRight,
                     ORZ_WINDOWS_RUN_USER env, headless profile + NTUSER.DAT).
    Stage control  - enforcement-probe -Arm control on the unhardened baseline.
    Stage nonadmin - apply non-admin template, enforcement-probe -Arm non-admin.
    Stage highnist - revert non-admin, apply high-nist template
                     (DeepSeek pin + AppLocker restore), enforcement-probe.
    Stage taskcontrol - baseline restore -> sync -> setup -> P0-0l-④ task-set
                     control-arm baseline run (k=1) + evidence collect.
    Stage tasknonadmin - baseline restore -> sync -> setup -> non-admin
                     template -> enforcement-probe wall gate -> task-set
                     run (k=1) + evidence collect.
    Stage taskhighnist - baseline restore -> sync -> setup -> high-nist
                     template (DeepSeek pin + AppLocker) -> wall probe ->
                     task-set run (k=1) + evidence collect.
    Stage agentcontrol - network/agent baseline restore (S4-BASE-NET-
                     2026-09-02, carries the AgentUser credential) -> sync
                     -> setup -> run_agent_arm.ps1 control arm (orz agent,
                     credential bootstrap + process-env injection).
    Stage agenthighnist - network/agent baseline restore -> sync -> setup
                     -> high-nist template (DeepSeek pin + AppLocker) ->
                     wall probe -> run_agent_arm.ps1 high-nist arm
                     (credential bootstrap + --env-file injection +
                     allowlist 221.204.163.76).
    Stage netcheck  - live VM only (NO checkpoint restore): inspects the
                     Clash/mihomo/verge process, config dirs, listening
                     ports and outbound connectivity through the proxy.
                     Evidence copied to evidence-netcheck/.
    Stage wrapup    - checkpoint restore (default S4-BASE-NET-2026-09-02,
                     the network/agent baseline) -> sync -> setup ->
                     pre-apply cleanup + AgentUser direct CredReadW ->
                     apply high-nist (DeepSeek pin + AppLocker) ->
                     AppLocker re-verify + immediate enforcement probe
                     (LoadUserProfileW 5023 regression check, twice) ->
                     CredReadW inside sandbox (non-admin + high-nist) ->
                     evidence collect.
    Stage all      - sync -> setup -> control -> nonadmin -> highnist.
    Stage full     - restore -> sync -> setup -> control -> nonadmin -> highnist.

    Fail-closed: any job exit != 0 aborts the remaining stages ('all' mode)
    and the driver exits nonzero.

    Checkpoint selection: -CheckpointName overrides the per-stage default.
    Stages that re-create the network/agent baseline (wrapup today; future
    agent rounds) default to S4-BASE-NET-2026-09-02; machine-side control
    stages keep S4-BASE-INSTALLED.  Pass -CheckpointName S4-BASE-INSTALLED
    to force the pre-network baseline for a net-default stage.
    ASCII only.
#>
[CmdletBinding()]
param(
    [ValidateSet('restore', 'sync', 'setup', 'control', 'nonadmin', 'highnist', 'taskcontrol', 'tasknonadmin', 'taskhighnist', 'agentcontrol', 'agenthighnist', 'netcheck', 'wrapup', 'all', 'full', 'reboot')]
    [string]$Stage = 'all',
    [string]$OutDir = 'D:\CLI\_windows_high_nist\formal-2026-09-02',
    [string]$CheckpointName = ''
)

$ErrorActionPreference = 'Stop'
$vmName = 'win-s4'
$credUser = 'HL'
$credPass = '123456'
$runner = 'D:\CLI\scripts\s4_vm_run_elev.ps1'
$outFile = Join-Path $OutDir ("stage-$Stage.txt")

$checkpointAllowed = @('S4-BASE-INSTALLED', 'S4-BASE-NET-2026-09-02')
$checkpointNetStages = @('wrapup', 'agentcontrol', 'agenthighnist')

function Resolve-CheckpointName {
    param(
        [string]$Requested,
        [string]$ForStage
    )
    $name = $Requested
    if ([string]::IsNullOrWhiteSpace($name)) {
        if ($checkpointNetStages -contains $ForStage) {
            $name = 'S4-BASE-NET-2026-09-02'
        }
        else {
            $name = 'S4-BASE-INSTALLED'
        }
    }
    if ($checkpointAllowed -notcontains $name) {
        throw "unsupported checkpoint name '$name' (allowed: $($checkpointAllowed -join ', '))"
    }
    return $name
}

$checkpoint = Resolve-CheckpointName -Requested $CheckpointName -ForStage $Stage
Add-Content -LiteralPath $outFile -Value "CHECKPOINT_RESOLVED=$checkpoint" -Encoding utf8

if (-not (Test-Path -LiteralPath $OutDir)) {
    New-Item -ItemType Directory -Path $OutDir -Force | Out-Null
}

$runIdentity = [System.Security.Principal.WindowsIdentity]::GetCurrent()
$runPrincipal = [System.Security.Principal.WindowsPrincipal]::new($runIdentity)
$isAdmin = $runPrincipal.IsInRole([System.Security.Principal.WindowsBuiltInRole]::Administrator)
Add-Content -LiteralPath $outFile -Value "RUN_USER=$($runIdentity.Name)" -Encoding utf8
Add-Content -LiteralPath $outFile -Value "RUN_ISADMIN=$isAdmin" -Encoding utf8

$lines = New-Object System.Collections.Generic.List[string]
function Write-Both([string]$msg) {
    $lines.Add($msg)
    Write-Output $msg
}
function Write-Exit([string]$msg) {
    $lines.Add($msg)
    $lines | Set-Content -LiteralPath $outFile -Encoding utf8
}

function Get-VmCredential {
    $sec = ConvertTo-SecureString $credPass -AsPlainText -Force
    return New-Object System.Management.Automation.PSCredential($credUser, $sec)
}

function Wait-VmReachable {
    param([int]$TimeoutSeconds = 300)
    $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
    while ((Get-Date) -lt $deadline) {
        try {
            $t = New-PSSession -VMName $vmName -Credential (Get-VmCredential) -ErrorAction Stop
            Remove-PSSession -Session $t -ErrorAction SilentlyContinue
            Write-Both "VM_REACHABLE=yes"
            return
        }
        catch {
            Start-Sleep -Seconds 10
        }
    }
    throw "win-s4 not reachable within $TimeoutSeconds seconds"
}

function Invoke-GuestJob {
    param(
        [string]$Name,
        [string]$Body,
        [int]$TimeoutSeconds = 600,
        [int]$RetryAttempts = 1
    )
    $bodyFile = Join-Path $OutDir ("job-$Name-body.ps1")
    $resFile = Join-Path $OutDir ("job-$Name-result.txt")
    $attempt = 0
    $ok = $false
    while ($attempt -le $RetryAttempts) {
        $attempt++
        Set-Content -LiteralPath $bodyFile -Value $Body -Encoding utf8
        Remove-Item -LiteralPath $resFile -Force -ErrorAction SilentlyContinue
        & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $runner `
            -JobBodyFile $bodyFile -OutFile $resFile -TimeoutSeconds $TimeoutSeconds
        $runnerExit = $LASTEXITCODE
        $resFailed = $false
        $hasExitLine = $false
        if (Test-Path -LiteralPath $resFile) {
            $errHit = Get-Content -LiteralPath $resFile -Encoding UTF8 |
                Where-Object { $_ -match '^(ERROR:|JOB_TIMEOUT)' } | Select-Object -First 1
            if ($errHit) {
                $resFailed = $true
            }
            else {
                $exitHit = Get-Content -LiteralPath $resFile -Encoding UTF8 |
                    Where-Object { $_ -match '^EXIT=(-?\d+)$' } | Select-Object -Last 1
                if ($exitHit) {
                    $hasExitLine = $true
                }
            }
        }
        if ($runnerExit -eq 0 -and $hasExitLine -and -not $resFailed) {
            $ok = $true
            break
        }
        $lines.Add("GUEST_JOB_ATTEMPT=$Name attempt=$attempt runnerExit=$runnerExit resFailed=$resFailed")
        Write-Host "GUEST_JOB_ATTEMPT=$Name attempt=$attempt runnerExit=$runnerExit resFailed=$resFailed"
        if ($attempt -le $RetryAttempts) {
            $lines.Add("GUEST_JOB_RETRY=$Name in 10s")
            Write-Host "GUEST_JOB_RETRY=$Name in 10s"
            Start-Sleep -Seconds 10
        }
    }
    $lines.Add("===== JOB $Name =====")
    Write-Host "===== JOB $Name ====="
    $exitCode = -1
    if (Test-Path -LiteralPath $resFile) {
        $resText = Get-Content -LiteralPath $resFile -Encoding UTF8
        foreach ($l in $resText) {
            $lines.Add($l)
        }
        $lastExit = $resText | Where-Object { $_ -match '^EXIT=(\d+)$' } | Select-Object -Last 1
        if ($lastExit -and $lastExit -match '^EXIT=(\d+)$') {
            $exitCode = [int]$Matches[1]
        }
    }
    if (-not $ok) {
        $exitCode = -1
    }
    $lines.Add("JOB_EXIT=$exitCode")
    Write-Host "JOB_EXIT=$exitCode"
    return $exitCode
}

function Sync-Harness {
    $sess = New-PSSession -VMName $vmName -Credential (Get-VmCredential)
    try {
        Invoke-Command -Session $sess -ScriptBlock {
            foreach ($d in @(
                'C:\s4\assurance',
                'C:\s4\scripts',
                'C:\s4\_windows_high_nist\hardening',
                'C:\s4\_windows_high_nist\policy',
                'C:\s4\_windows_high_nist\run',
                'C:\s4\_windows_high_nist\agent-tasks-tb2.1',
                'C:\workspace'
            )) {
                New-Item -ItemType Directory -Path $d -Force | Out-Null
            }
        }
        $targets = @(
            @{ Src = 'D:\CLI\assurance\windows_sandbox.py'; Dst = 'C:\s4\assurance\windows_sandbox.py' },
            @{ Src = 'D:\CLI\assurance\sandbox_verifier.py'; Dst = 'C:\s4\assurance\sandbox_verifier.py' },
            @{ Src = 'D:\CLI\assurance\windows-native-sandbox-run-v0.1.schema.json'; Dst = 'C:\s4\assurance\windows-native-sandbox-run-v0.1.schema.json' },
            @{ Src = 'D:\CLI\assurance\windows-native-sandbox-observation-v0.1.schema.json'; Dst = 'C:\s4\assurance\windows-native-sandbox-observation-v0.1.schema.json' },
            @{ Src = 'D:\CLI\assurance\windows-native-sandbox-profile-v0.1.json'; Dst = 'C:\s4\assurance\windows-native-sandbox-profile-v0.1.json' },
            @{ Src = 'D:\CLI\assurance\windows-native-sandbox-profile-v0.1.schema.json'; Dst = 'C:\s4\assurance\windows-native-sandbox-profile-v0.1.schema.json' },
            @{ Src = 'D:\CLI\scripts\run_windows_native_sandbox_command.py'; Dst = 'C:\s4\scripts\run_windows_native_sandbox_command.py' },
            @{ Src = 'D:\CLI\_windows_high_nist\hardening\apply_hardening.ps1'; Dst = 'C:\s4\_windows_high_nist\hardening\apply_hardening.ps1' },
            @{ Src = 'D:\CLI\_windows_high_nist\policy\enforcement_probe.ps1'; Dst = 'C:\s4\_windows_high_nist\policy\enforcement_probe.ps1' },
            @{ Src = 'D:\CLI\_windows_high_nist\policy\enforcement_probe.py'; Dst = 'C:\s4\_windows_high_nist\policy\enforcement_probe.py' },
            @{ Src = 'D:\CLI\_windows_high_nist\run\run_enforcement_probe.ps1'; Dst = 'C:\s4\_windows_high_nist\run\run_enforcement_probe.ps1' },
            @{ Src = 'D:\CLI\_windows_high_nist\run\run_task_arm.ps1'; Dst = 'C:\s4\_windows_high_nist\run\run_task_arm.ps1' },
            @{ Src = 'D:\CLI\_windows_high_nist\run\run_agent_arm.ps1'; Dst = 'C:\s4\_windows_high_nist\run\run_agent_arm.ps1' },
            @{ Src = 'D:\CLI\_windows_high_nist\tools\cred_probe.exe'; Dst = 'C:\s4\tools\cred_probe.exe' },
            @{ Src = 'D:\CLI\orz\target\release\orz.exe'; Dst = 'C:\s4\tools\orz.exe' }
        )
        $allOk = $true
        foreach ($t in $targets) {
            Copy-Item -LiteralPath $t.Src -Destination $t.Dst -ToSession $sess -Force
            $srcHash = (Get-FileHash -LiteralPath $t.Src -Algorithm SHA256).Hash
            $dstHash = Invoke-Command -Session $sess -ArgumentList $t.Dst -ScriptBlock {
                param($p) (Get-FileHash -LiteralPath $p -Algorithm SHA256).Hash
            }
            $ok = ($srcHash -eq $dstHash)
            if (-not $ok) { $allOk = $false }
            Write-Both "SYNC $($t.Dst) ok=$ok"
        }
        try {
            Copy-Item -LiteralPath 'D:\CLI\_windows_high_nist\tasks' `
                -Destination 'C:\s4\_windows_high_nist' -Recurse -Force -ToSession $sess
            $taskCount = Invoke-Command -Session $sess -ArgumentList 'C:\s4\_windows_high_nist\tasks' -ScriptBlock {
                param($p)
                @(Get-ChildItem -LiteralPath $p -Filter 'task.json' -Recurse -File).Count
            }
            $hostManifest = Get-Content -LiteralPath 'D:\CLI\_windows_high_nist\tasks\manifest.json' -Raw -Encoding UTF8 |
                ConvertFrom-Json
            $expectedTasks = @($hostManifest.tasks).Count
            Write-Both "SYNC_TASKS count=$taskCount"
            Write-Both "SYNC_TASKS expected=$expectedTasks"
            if ($taskCount -ne $expectedTasks) { $allOk = $false }
        }
        catch {
            $allOk = $false
            Write-Both "SYNC_TASKS error=$($_.Exception.Message)"
        }
        try {
            $tbSrc = 'D:\CLI\_windows_high_nist\agent-tasks-tb2.1'
            if (-not (Test-Path -LiteralPath $tbSrc)) {
                throw "agent task set missing: $tbSrc"
            }
            Copy-Item -LiteralPath $tbSrc `
                -Destination 'C:\s4\_windows_high_nist' -Recurse -Force -ToSession $sess
            $tbCount = Invoke-Command -Session $sess -ArgumentList 'C:\s4\_windows_high_nist\agent-tasks-tb2.1' -ScriptBlock {
                param($p)
                @(Get-ChildItem -LiteralPath $p -Filter 'task.json' -Recurse -File).Count
            }
            $tbManifest = Get-Content -LiteralPath (Join-Path $tbSrc 'manifest.json') -Raw -Encoding UTF8 |
                ConvertFrom-Json
            $tbExpected = @($tbManifest.tasks).Count
            Write-Both "SYNC_AGENT_TASKS_TB count=$tbCount"
            Write-Both "SYNC_AGENT_TASKS_TB expected=$tbExpected"
            if ($tbCount -ne $tbExpected) { $allOk = $false }
        }
        catch {
            $allOk = $false
            Write-Both "SYNC_AGENT_TASKS_TB error=$($_.Exception.Message)"
        }
        Write-Both "SYNC_ALL_OK=$allOk"
        if (-not $allOk) {
            throw 'harness sync hash mismatch'
        }
    }
    finally {
        Remove-PSSession -Session $sess -ErrorAction SilentlyContinue
    }
}

function Collect-VmEvidence {
    param([string]$Arm)
    $sess = New-PSSession -VMName $vmName -Credential (Get-VmCredential)
    try {
        $evDir = Join-Path $OutDir "evidence-$Arm"
        New-Item -ItemType Directory -Path $evDir -Force | Out-Null
        foreach ($f in @("enforcement-probe-$Arm.json", "windows-native-run-observation-$Arm.json")) {
            $src = "C:\workspace\$Arm\$f"
            $dst = Join-Path $evDir $f
            $exists = Invoke-Command -Session $sess -ArgumentList $src -ScriptBlock {
                param($p) Test-Path -LiteralPath $p
            }
            if ($exists) {
                Copy-Item -LiteralPath $src -Destination $dst -FromSession $sess -Force
                Write-Both "EVIDENCE_COPIED=$dst"
            }
            else {
                Write-Both "EVIDENCE_MISSING=$src"
            }
        }
    }
    finally {
        Remove-PSSession -Session $sess -ErrorAction SilentlyContinue
    }
}

function Collect-TaskEvidence {
    param(
        [string]$ArmLabel,
        [string]$GuestWorkspace = 'C:\workspace\taskcontrol',
        [string]$NameFilter = ''
    )
    $sess = New-PSSession -VMName $vmName -Credential (Get-VmCredential)
    try {
        $evDir = Join-Path $OutDir "evidence-$ArmLabel"
        New-Item -ItemType Directory -Path $evDir -Force | Out-Null
        $srcDir = $GuestWorkspace
        $filter = $NameFilter
        if (-not $filter) {
            $filter = '^(agent-baseline-|agent-|cred-bootstrap-|journal-|obs-|task-baseline|task-outcome-|run-observation-|verify-|enforcement-probe-|windows-native-run-observation-)'
        }
        $names = @(Invoke-Command -Session $sess -ArgumentList $srcDir, $filter -ScriptBlock {
            param($p, $f)
            $found = @(Get-ChildItem -LiteralPath $p -Filter '*.json' -File -ErrorAction SilentlyContinue |
                Select-Object -ExpandProperty Name |
                Where-Object { $_ -match $f })
            $found += @(Get-ChildItem -LiteralPath $p -Filter '*.jsonl' -File -ErrorAction SilentlyContinue |
                Select-Object -ExpandProperty Name |
                Where-Object { $_ -match $f })
            $found
        })
        foreach ($n in $names) {
            $src = Join-Path $srcDir $n
            $dst = Join-Path $evDir $n
            Copy-Item -LiteralPath $src -Destination $dst -FromSession $sess -Force
            Write-Both "EVIDENCE_COPIED=$dst"
        }
        if ($names.Count -eq 0) {
            Write-Both 'EVIDENCE_TASK_NONE'
        }
    }
    finally {
        Remove-PSSession -Session $sess -ErrorAction SilentlyContinue
    }
}

# --- task-arm stage (self-contained: baseline restore -> sync -> setup ->
#     arm job -> evidence collect) -----------------------------------------

function Invoke-TaskArmStage {
    param(
        [string]$StageName,
        [string]$JobName,
        [string]$Body,
        [int]$JobTimeoutSeconds = 1800,
        [string]$ArmLabel,
        [string]$GuestWorkspace,
        [string]$SnapshotName = 'S4-BASE-INSTALLED'
    )
    Write-Both "STAGE=$StageName"
    $vm = Get-VM -Name $vmName -ErrorAction Stop
    Write-Both "STATE_BEFORE=$($vm.State)"
    if ($vm.State -ne 'Off' -and $vm.State -ne 'Saved') {
        Write-Both 'ACTION=stop-vm'
        Stop-VM -Name $vmName -ErrorAction Stop
        $deadline = (Get-Date).AddMinutes(3)
        while ((Get-Date) -lt $deadline) {
            Start-Sleep -Seconds 5
            $cur = Get-VM -Name $vmName -ErrorAction SilentlyContinue
            if ($null -ne $cur -and ($cur.State -eq 'Off' -or $cur.State -eq 'Saved')) {
                break
            }
        }
        $vm = Get-VM -Name $vmName
        Write-Both "STATE_AFTER_STOP=$($vm.State)"
        if ($vm.State -ne 'Off' -and $vm.State -ne 'Saved') {
            throw 'VM did not stop'
        }
    }
    $snap = Get-VMSnapshot -VMName $vmName -Name $SnapshotName -ErrorAction Stop
    if (-not $snap) {
        throw "baseline checkpoint $SnapshotName missing"
    }
    Write-Both "CHECKPOINT=$($snap.Name) created=$($snap.CreationTime)"
    Restore-VMCheckpoint -VMName $vmName -Name $SnapshotName -Confirm:$false -ErrorAction Stop
    Write-Both 'RESTORE_OK=1'
    Start-VM -Name $vmName -ErrorAction Stop
    Write-Both 'START_ISSUED=1'
    Wait-VmReachable
    Sync-Harness
    $code = Invoke-GuestJob -Name 'formal-setup' -Body $setupBody -TimeoutSeconds 300
    if ($code -ne 0) {
        throw "formal setup job failed (exit $code)"
    }
    $code = Invoke-GuestJob -Name $JobName -Body $Body -TimeoutSeconds $JobTimeoutSeconds
    if ($code -ne 0) {
        throw "$JobName failed (exit $code)"
    }
    Collect-TaskEvidence -ArmLabel $ArmLabel -GuestWorkspace $GuestWorkspace
}

# --- job bodies --------------------------------------------------------------

$setupBody = @'
$env:Path = [Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [Environment]::GetEnvironmentVariable('Path','User')
$env:PYTHONPATH = 'C:\s4'
$py = 'C:\Program Files\Python312\python.exe'
$pw = 'S4Run!2026'

if (-not (Get-LocalUser -Name 'AgentUser' -ErrorAction SilentlyContinue)) {
    & net.exe user AgentUser $pw /add 2>&1 | ForEach-Object { Write-Output "NET_CREATE: $_" }
    Write-Output "NET_CREATE_EXIT=$LASTEXITCODE"
}
& net.exe user AgentUser $pw 2>&1 | ForEach-Object { Write-Output "NET_PW: $_" }
Write-Output "NET_PW_EXIT=$LASTEXITCODE"

[Environment]::SetEnvironmentVariable('ORZ_WINDOWS_RUN_USER', 'AgentUser', 'Machine')
[Environment]::SetEnvironmentVariable('ORZ_WINDOWS_RUN_USER_PASSWORD', $pw, 'Machine')
[Environment]::SetEnvironmentVariable('ORZ_WINDOWS_RUN_USER_DOMAIN', $env:COMPUTERNAME, 'Machine')
Write-Output "ENV_USER=$([Environment]::GetEnvironmentVariable('ORZ_WINDOWS_RUN_USER','Machine'))"
Write-Output "ENV_DOMAIN=$([Environment]::GetEnvironmentVariable('ORZ_WINDOWS_RUN_USER_DOMAIN','Machine'))"

$cfg = 'C:\s4\tools\secpol.cfg'
$sdb = 'C:\s4\tools\secpol.sdb'
Remove-Item -LiteralPath $cfg,$sdb -Force -ErrorAction SilentlyContinue
& secedit.exe /export /cfg $cfg /areas USER_RIGHTS 2>&1 | ForEach-Object { Write-Output "SE_EXPORT: $_" }
$u = [ADSI]"WinNT://$env:COMPUTERNAME/AgentUser"
$sidBytes = $u.objectSID[0]
$sid = [System.Security.Principal.SecurityIdentifier]::new($sidBytes, 0).Value
Write-Output "AGENTUSER_SID=$sid"
$cfgLines = Get-Content -LiteralPath $cfg
$outLines = New-Object System.Collections.Generic.List[string]
$patched = $false
foreach ($line in $cfgLines) {
    if ($line -match '^SeBatchLogonRight\s*=') {
        $outLines.Add($line.TrimEnd() + ",*$sid")
        $patched = $true
    }
    else {
        $outLines.Add($line)
    }
}
if (-not $patched) {
    $outLines.Add("SeBatchLogonRight = *$sid")
}
$outLines | Set-Content -LiteralPath $cfg -Encoding ascii
& secedit.exe /configure /db $sdb /cfg $cfg /areas USER_RIGHTS 2>&1 | ForEach-Object { Write-Output "SE_CONF: $_" }
Write-Output "SECEDIT_EXIT=$LASTEXITCODE"
$env:GSA_AGENT_SID = $sid

$profCode = @"
import ctypes, os, subprocess
from ctypes import wintypes
advapi32 = ctypes.WinDLL('advapi32', use_last_error=True)
userenv = ctypes.WinDLL('userenv', use_last_error=True)
advapi32.LogonUserW.argtypes = [wintypes.LPCWSTR, wintypes.LPCWSTR, wintypes.LPCWSTR, wintypes.DWORD, wintypes.DWORD, ctypes.POINTER(wintypes.HANDLE)]
advapi32.LogonUserW.restype = wintypes.BOOL
class PI(ctypes.Structure):
    _fields_ = [('dwSize', wintypes.DWORD), ('dwFlags', wintypes.DWORD), ('lpUserName', wintypes.LPWSTR), ('lpProfilePath', wintypes.LPWSTR), ('lpDefaultPath', wintypes.LPWSTR), ('lpServerName', wintypes.LPWSTR), ('lpPolicyPath', wintypes.LPWSTR), ('hProfile', wintypes.HANDLE)]
userenv.LoadUserProfileW.argtypes = [wintypes.HANDLE, ctypes.POINTER(PI)]
userenv.LoadUserProfileW.restype = wintypes.BOOL
userenv.UnloadUserProfile.argtypes = [wintypes.HANDLE, wintypes.HANDLE]
userenv.UnloadUserProfile.restype = wintypes.BOOL
sid = os.environ.get('GSA_AGENT_SID', '')
tok = wintypes.HANDLE()
ok = advapi32.LogonUserW('AgentUser', os.environ.get('COMPUTERNAME', ''), 'S4Run!2026', 4, 0, ctypes.byref(tok))
print('PROF_LOGON', ok, ctypes.get_last_error())
info = PI()
info.dwSize = ctypes.sizeof(PI)
info.lpUserName = wintypes.LPWSTR('AgentUser')
okl = userenv.LoadUserProfileW(tok, ctypes.byref(info))
print('PROF_LOAD', okl, ctypes.get_last_error(), 'hProfile', info.hProfile)
ntuser = r'C:\Users\AgentUser\NTUSER.DAT'
print('NTUSER_EXISTS1', os.path.exists(ntuser))
if okl and info.hProfile:
    if not os.path.exists(ntuser) and sid:
        r = subprocess.run(['reg.exe', 'save', 'HKU\\' + sid, ntuser, '/y'], capture_output=True)
        print('REG_SAVE', r.returncode, r.stdout.decode('utf-8', 'replace').strip(), r.stderr.decode('utf-8', 'replace').strip())
    userenv.UnloadUserProfile(tok, info.hProfile)
print('NTUSER_EXISTS2', os.path.exists(ntuser))
print('PROF_DONE')
"@
Set-Content -LiteralPath 'C:\s4\tools\mkprofile_formal.py' -Value $profCode -Encoding utf8
& $py 'C:\s4\tools\mkprofile_formal.py' 2>&1 | ForEach-Object { Write-Output "PROF: $_" }

$nt = 'C:\Users\AgentUser\NTUSER.DAT'
Write-Output "NTUSER_FINAL=$(Test-Path -LiteralPath $nt)"
$profKey = "HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion\ProfileList\$sid"
& reg.exe add $profKey /v ProfileImagePath /t REG_EXPAND_SZ /d 'C:\Users\AgentUser' /f 2>&1 | ForEach-Object { Write-Output "PIP: $_" }
& icacls.exe 'C:\Users\AgentUser' /grant 'AgentUser:(OI)(CI)(M)' /T 2>&1 | Out-Null
Write-Output "PROF_ICACLS_EXIT=$LASTEXITCODE"
& icacls.exe 'C:\WINDOWS\system32\config\AgentUser' /remove:d 'AgentUser' /T /C 2>&1 | Out-Null
Write-Output "WRONGPATH_CLEAN_EXIT=$LASTEXITCODE"

if (-not $sid) {
    Write-Output 'SETUP_SID_MISSING'
    exit 1
}
if (-not (Test-Path -LiteralPath $nt)) {
    Write-Output 'SETUP_NTUSER_MISSING'
    exit 1
}
Write-Output 'SETUP_OK'
exit 0
'@

$controlBody = @'
$env:Path = [Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [Environment]::GetEnvironmentVariable('Path','User')
$env:PYTHONPATH = 'C:\s4'
$run = 'C:\s4\_windows_high_nist\run\run_enforcement_probe.ps1'
New-Item -ItemType Directory -Path 'C:\workspace\control' -Force | Out-Null
Write-Output '===== CONTROL PROBE ====='
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File $run -Arm control -Workspace C:\workspace\control -ResultPath C:\workspace\control\enforcement-probe-control.json -TimeoutSeconds 180 2>&1 | ForEach-Object { Write-Output "CTL: $_" }
$probeExit = $LASTEXITCODE
Write-Output "CTL_PROBE_EXIT=$probeExit"
$obs = 'C:\workspace\control\windows-native-run-observation-control.json'
if (Test-Path -LiteralPath $obs) {
    $o = Get-Content -LiteralPath $obs -Raw | ConvertFrom-Json
    Write-Output "CTL_OBS outcome=$($o.outcome) exit=$($o.process.exit_code)"
    if ($o.diagnostics) { $o.diagnostics | ForEach-Object { Write-Output "CTL_DIAG: $_" } }
    if ($o.checks) { $o.checks.PSObject.Properties | ForEach-Object { Write-Output "CTL_CHECK $($_.Name)=$($_.Value)" } }
}
if (Test-Path -LiteralPath 'C:\workspace\control\enforcement-probe-control.json') {
    Get-Content -LiteralPath 'C:\workspace\control\enforcement-probe-control.json' -Raw
}
exit $probeExit
'@

$nonadminBody = @'
$env:Path = [Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [Environment]::GetEnvironmentVariable('Path','User')
$env:PYTHONPATH = 'C:\s4'
$hard = 'C:\s4\_windows_high_nist\hardening\apply_hardening.ps1'
$run = 'C:\s4\_windows_high_nist\run\run_enforcement_probe.ps1'
New-Item -ItemType Directory -Path 'C:\workspace\non-admin' -Force | Out-Null
Write-Output '===== APPLY NON-ADMIN ====='
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File $hard -Arm non-admin -RunUser AgentUser -WorkspaceRoot C:\workspace 2>&1 | ForEach-Object { Write-Output "NAH: $_" }
Write-Output "NAH_EXIT=$LASTEXITCODE"
if ($LASTEXITCODE -ne 0) {
    Write-Output 'NONADMIN_APPLY_FAILED'
    exit 1
}
Write-Output '===== NON-ADMIN PROBE ====='
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File $run -Arm non-admin -Workspace C:\workspace\non-admin -ResultPath C:\workspace\non-admin\enforcement-probe-non-admin.json -TimeoutSeconds 180 2>&1 | ForEach-Object { Write-Output "NA: $_" }
$probeExit = $LASTEXITCODE
Write-Output "NA_PROBE_EXIT=$probeExit"
$obs = 'C:\workspace\non-admin\windows-native-run-observation-non-admin.json'
if (Test-Path -LiteralPath $obs) {
    $o = Get-Content -LiteralPath $obs -Raw | ConvertFrom-Json
    Write-Output "NA_OBS outcome=$($o.outcome) exit=$($o.process.exit_code)"
    if ($o.diagnostics) { $o.diagnostics | ForEach-Object { Write-Output "NA_DIAG: $_" } }
    if ($o.checks) { $o.checks.PSObject.Properties | ForEach-Object { Write-Output "NA_CHECK $($_.Name)=$($_.Value)" } }
}
if (Test-Path -LiteralPath 'C:\workspace\non-admin\enforcement-probe-non-admin.json') {
    Get-Content -LiteralPath 'C:\workspace\non-admin\enforcement-probe-non-admin.json' -Raw
}
exit $probeExit
'@

$highnistBody = @'
$env:Path = [Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [Environment]::GetEnvironmentVariable('Path','User')
$env:PYTHONPATH = 'C:\s4'
$hard = 'C:\s4\_windows_high_nist\hardening\apply_hardening.ps1'
$run = 'C:\s4\_windows_high_nist\run\run_enforcement_probe.ps1'
New-Item -ItemType Directory -Path 'C:\workspace\high-nist' -Force | Out-Null
Write-Output '===== REVERT NON-ADMIN ====='
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File $hard -Arm non-admin -Revert -RunUser AgentUser -WorkspaceRoot C:\workspace 2>&1 | ForEach-Object { Write-Output "REVNA: $_" }
Write-Output "REVNA_EXIT=$LASTEXITCODE"
Write-Output '===== APPLY HIGH-NIST (DeepSeek pin + AppLocker restore) ====='
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File $hard -Arm high-nist -RunUser AgentUser -WorkspaceRoot C:\workspace -DeepSeekIp 221.204.163.76 -EnableAppLocker 2>&1 | ForEach-Object { Write-Output "HNH: $_" }
Write-Output "HNH_EXIT=$LASTEXITCODE"
if ($LASTEXITCODE -ne 0) {
    Write-Output 'HIGHNIST_APPLY_FAILED'
    exit 1
}
Write-Output '===== HIGH-NIST PROBE ====='
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File $run -Arm high-nist -Workspace C:\workspace\high-nist -ResultPath C:\workspace\high-nist\enforcement-probe-high-nist.json -TimeoutSeconds 180 2>&1 | ForEach-Object { Write-Output "HN: $_" }
$probeExit = $LASTEXITCODE
Write-Output "HN_PROBE_EXIT=$probeExit"
$obs = 'C:\workspace\high-nist\windows-native-run-observation-high-nist.json'
if (Test-Path -LiteralPath $obs) {
    $o = Get-Content -LiteralPath $obs -Raw | ConvertFrom-Json
    Write-Output "HN_OBS outcome=$($o.outcome) exit=$($o.process.exit_code)"
    if ($o.diagnostics) { $o.diagnostics | ForEach-Object { Write-Output "HN_DIAG: $_" } }
    if ($o.checks) { $o.checks.PSObject.Properties | ForEach-Object { Write-Output "HN_CHECK $($_.Name)=$($_.Value)" } }
}
if (Test-Path -LiteralPath 'C:\workspace\high-nist\enforcement-probe-high-nist.json') {
    Get-Content -LiteralPath 'C:\workspace\high-nist\enforcement-probe-high-nist.json' -Raw
}
exit $probeExit
'@

$taskcontrolBody = @'
$env:Path = [Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [Environment]::GetEnvironmentVariable('Path','User')
$env:PYTHONPATH = 'C:\s4'
$run = 'C:\s4\_windows_high_nist\run\run_task_arm.ps1'
New-Item -ItemType Directory -Path 'C:\workspace\taskcontrol' -Force | Out-Null
Write-Output '===== TASK CONTROL BASELINE ====='
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File $run -Arm control `
    -Workspace C:\workspace\taskcontrol `
    -ResultPath C:\workspace\taskcontrol\task-baseline-control.json 2>&1 |
    ForEach-Object { Write-Output "TASK: $_" }
$taskExit = $LASTEXITCODE
Write-Output "TASK_CONTROL_EXIT=$taskExit"
if (Test-Path -LiteralPath 'C:\workspace\taskcontrol\task-baseline-control.json') {
    Get-Content -LiteralPath 'C:\workspace\taskcontrol\task-baseline-control.json' -Raw
}
exit $taskExit
'@

$tasknonadminBody = @'
$env:Path = [Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [Environment]::GetEnvironmentVariable('Path','User')
$env:PYTHONPATH = 'C:\s4'
$hard = 'C:\s4\_windows_high_nist\hardening\apply_hardening.ps1'
$probeRun = 'C:\s4\_windows_high_nist\run\run_enforcement_probe.ps1'
$taskRun = 'C:\s4\_windows_high_nist\run\run_task_arm.ps1'
$ws = 'C:\workspace\tasknonadmin'
New-Item -ItemType Directory -Path $ws -Force | Out-Null
Write-Output '===== APPLY NON-ADMIN ====='
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File $hard -Arm non-admin -RunUser AgentUser -WorkspaceRoot C:\workspace 2>&1 | ForEach-Object { Write-Output "TNAH: $_" }
Write-Output "TNAH_EXIT=$LASTEXITCODE"
if ($LASTEXITCODE -ne 0) {
    Write-Output 'TASKNONADMIN_APPLY_FAILED'
    exit 1
}
Write-Output '===== NON-ADMIN WALL PROBE ====='
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File $probeRun -Arm non-admin -Workspace $ws -ResultPath (Join-Path $ws 'enforcement-probe-non-admin.json') -TimeoutSeconds 180 2>&1 | ForEach-Object { Write-Output "TNAP: $_" }
$probeExit = $LASTEXITCODE
Write-Output "TNA_PROBE_EXIT=$probeExit"
if ($probeExit -ne 0) {
    Write-Output 'TASKNONADMIN_PROBE_FAILED'
    exit 1
}
Write-Output '===== NON-ADMIN TASK BATCH ====='
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File $taskRun -Arm non-admin -Workspace $ws -ResultPath (Join-Path $ws 'task-baseline-non-admin.json') 2>&1 | ForEach-Object { Write-Output "TNAT: $_" }
$taskExit = $LASTEXITCODE
Write-Output "TASK_NONADMIN_EXIT=$taskExit"
if (Test-Path -LiteralPath (Join-Path $ws 'task-baseline-non-admin.json')) {
    Get-Content -LiteralPath (Join-Path $ws 'task-baseline-non-admin.json') -Raw
}
exit $taskExit
'@

$taskhighnistBody = @'
$env:Path = [Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [Environment]::GetEnvironmentVariable('Path','User')
$env:PYTHONPATH = 'C:\s4'
$hard = 'C:\s4\_windows_high_nist\hardening\apply_hardening.ps1'
$probeRun = 'C:\s4\_windows_high_nist\run\run_enforcement_probe.ps1'
$taskRun = 'C:\s4\_windows_high_nist\run\run_task_arm.ps1'
$ws = 'C:\workspace\taskhighnist'
New-Item -ItemType Directory -Path $ws -Force | Out-Null
Write-Output '===== APPLY HIGH-NIST (DeepSeek pin + AppLocker restore) ====='
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File $hard -Arm high-nist -RunUser AgentUser -WorkspaceRoot C:\workspace -DeepSeekIp 221.204.163.76 -EnableAppLocker 2>&1 | ForEach-Object { Write-Output "THNH: $_" }
Write-Output "THNH_EXIT=$LASTEXITCODE"
if ($LASTEXITCODE -ne 0) {
    Write-Output 'TASKHIGHNIST_APPLY_FAILED'
    exit 1
}
Write-Output '===== HIGH-NIST WALL PROBE ====='
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File $probeRun -Arm high-nist -Workspace $ws -ResultPath (Join-Path $ws 'enforcement-probe-high-nist.json') -TimeoutSeconds 180 2>&1 | ForEach-Object { Write-Output "THNP: $_" }
$probeExit = $LASTEXITCODE
Write-Output "THN_PROBE_EXIT=$probeExit"
if ($probeExit -ne 0) {
    Write-Output 'TASKHIGHNIST_PROBE_FAILED'
    exit 1
}
Write-Output '===== HIGH-NIST TASK BATCH ====='
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File $taskRun -Arm high-nist -Workspace $ws -ResultPath (Join-Path $ws 'task-baseline-high-nist.json') 2>&1 | ForEach-Object { Write-Output "THNT: $_" }
$taskExit = $LASTEXITCODE
Write-Output "TASK_HIGHNIST_EXIT=$taskExit"
if (Test-Path -LiteralPath (Join-Path $ws 'task-baseline-high-nist.json')) {
    Get-Content -LiteralPath (Join-Path $ws 'task-baseline-high-nist.json') -Raw
}
exit $taskExit
'@

# --- agent-round job bodies (model-side k=1; 2026-09-03 wiring) -----------

$agentcontrolBody = @'
$env:Path = [Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [Environment]::GetEnvironmentVariable('Path','User')
$env:PYTHONPATH = 'C:\s4'
$orzSrc = 'C:\s4\tools\orz.exe'
$orzDst = 'C:\Program Files\orz\orz.exe'
if (Test-Path -LiteralPath $orzSrc) {
    Copy-Item -LiteralPath $orzSrc -Destination $orzDst -Force
    $h1 = (Get-FileHash -LiteralPath $orzSrc -Algorithm SHA256).Hash
    $h2 = (Get-FileHash -LiteralPath $orzDst -Algorithm SHA256).Hash
    Write-Output "AGENTCTRL_ORZ_SYNC_OK=$($h1 -eq $h2)"
}
$agentRun = 'C:\s4\_windows_high_nist\run\run_agent_arm.ps1'
$ws = 'C:\workspace\agentcontrol'
New-Item -ItemType Directory -Path $ws -Force | Out-Null
Write-Output '===== AGENT CONTROL BASELINE (credential bootstrap + process-env channel) ====='
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File $agentRun -Arm control `
    -Workspace $ws -ResultPath (Join-Path $ws 'agent-baseline-control.json') 2>&1 |
    ForEach-Object { Write-Output "AGENTCTRL: $_" }
$agentExit = $LASTEXITCODE
Write-Output "AGENT_CONTROL_EXIT=$agentExit"
if (Test-Path -LiteralPath (Join-Path $ws 'agent-baseline-control.json')) {
    Get-Content -LiteralPath (Join-Path $ws 'agent-baseline-control.json') -Raw
}
exit $agentExit
'@

$agenthighnistBody = @'
$env:Path = [Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [Environment]::GetEnvironmentVariable('Path','User')
$env:PYTHONPATH = 'C:\s4'
$orzSrc = 'C:\s4\tools\orz.exe'
$orzDst = 'C:\Program Files\orz\orz.exe'
if (Test-Path -LiteralPath $orzSrc) {
    Copy-Item -LiteralPath $orzSrc -Destination $orzDst -Force
    $h1 = (Get-FileHash -LiteralPath $orzSrc -Algorithm SHA256).Hash
    $h2 = (Get-FileHash -LiteralPath $orzDst -Algorithm SHA256).Hash
    Write-Output "AGHN_ORZ_SYNC_OK=$($h1 -eq $h2)"
}
$hard = 'C:\s4\_windows_high_nist\hardening\apply_hardening.ps1'
$probeRun = 'C:\s4\_windows_high_nist\run\run_enforcement_probe.ps1'
$agentRun = 'C:\s4\_windows_high_nist\run\run_agent_arm.ps1'
$ws = 'C:\workspace\agenthighnist'
New-Item -ItemType Directory -Path $ws -Force | Out-Null
Write-Output '===== APPLY HIGH-NIST (DeepSeek pin + AppLocker restore) ====='
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File $hard -Arm high-nist -RunUser AgentUser -WorkspaceRoot C:\workspace -DeepSeekIp 221.204.163.76 -EnableAppLocker 2>&1 | ForEach-Object { Write-Output "AGHNH: $_" }
Write-Output "AGHNH_EXIT=$LASTEXITCODE"
if ($LASTEXITCODE -ne 0) {
    Write-Output 'AGENTHIGHNIST_APPLY_FAILED'
    exit 1
}
Write-Output '===== HIGH-NIST WALL PROBE ====='
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File $probeRun -Arm high-nist -Workspace $ws -ResultPath (Join-Path $ws 'enforcement-probe-high-nist.json') -TimeoutSeconds 180 2>&1 | ForEach-Object { Write-Output "AGHNP: $_" }
$probeExit = $LASTEXITCODE
Write-Output "AGHN_PROBE_EXIT=$probeExit"
if ($probeExit -ne 0) {
    Write-Output 'AGENTHIGHNIST_PROBE_FAILED'
    exit 1
}
Write-Output '===== HIGH-NIST AGENT BATCH (bootstrap + --env-file + allowlist) ====='
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File $agentRun -Arm high-nist `
    -Workspace $ws -ResultPath (Join-Path $ws 'agent-baseline-high-nist.json') 2>&1 |
    ForEach-Object { Write-Output "AGHNTA: $_" }
$agentExit = $LASTEXITCODE
Write-Output "AGENT_HIGHNIST_EXIT=$agentExit"
if (Test-Path -LiteralPath (Join-Path $ws 'agent-baseline-high-nist.json')) {
    Get-Content -LiteralPath (Join-Path $ws 'agent-baseline-high-nist.json') -Raw
}
exit $agentExit
'@

# --- netcheck / wrapup job bodies -------------------------------------------

$netcheckBody = @'
$env:Path = [Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [Environment]::GetEnvironmentVariable('Path','User')
$ErrorActionPreference = 'Continue'
$outDir = 'C:\workspace\netcheck'
New-Item -ItemType Directory -Path $outDir -Force | Out-Null
$data = [ordered]@{
    ran_at = (Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ')
    processes = New-Object System.Collections.ArrayList
    config_dirs = New-Object System.Collections.ArrayList
    config_files = New-Object System.Collections.ArrayList
    ports = New-Object System.Collections.ArrayList
    listening = New-Object System.Collections.ArrayList
    curl = New-Object System.Collections.ArrayList
    direct_http = ''
    direct_exit = -1
}
$procs = @(Get-Process -ErrorAction SilentlyContinue | Where-Object { $_.ProcessName -match 'clash|mihomo|verge' })
foreach ($p in $procs) {
    [void]$data.processes.Add([ordered]@{ id = $p.Id; name = $p.ProcessName; session = $p.SessionId; path = $p.Path })
}
$proxyPids = @($data.processes | ForEach-Object { [int]$_.id })
$roots = @('C:\Users\HL\AppData\Roaming','C:\Users\HL\AppData\Local','C:\Users\HL\AppData\Local\Programs','C:\ProgramData','C:\Program Files','C:\Program Files (x86)')
foreach ($root in $roots) {
    if (-not (Test-Path -LiteralPath $root)) { continue }
    $dirs = @(Get-ChildItem -LiteralPath $root -Directory -ErrorAction SilentlyContinue | Where-Object { $_.Name -match 'clash|verge|mihomo' })
    foreach ($d in $dirs) {
        [void]$data.config_dirs.Add($d.FullName)
        foreach ($f in @(Get-ChildItem -LiteralPath $d.FullName -File -ErrorAction SilentlyContinue | Select-Object -First 25)) {
            [void]$data.config_files.Add("$($f.FullName)|$($f.Length)")
        }
        foreach ($s in @(Get-ChildItem -LiteralPath $d.FullName -Directory -ErrorAction SilentlyContinue | Select-Object -First 20)) {
            [void]$data.config_dirs.Add($s.FullName)
            foreach ($f in @(Get-ChildItem -LiteralPath $s.FullName -File -ErrorAction SilentlyContinue | Select-Object -First 25)) {
                [void]$data.config_files.Add("$($f.FullName)|$($f.Length)")
            }
        }
    }
}
$candidates = @($data.config_files | ForEach-Object { ($_ -split '\|')[0] } | Where-Object { $_ -match '\.ya?ml$' })
foreach ($c in $candidates) {
    try {
        $t = Get-Content -LiteralPath $c -Raw -ErrorAction Stop
        if ($t -match 'mixed-port:\s*["'']?(\d+)') { [void]$data.ports.Add([int]$Matches[1]) }
        if ($t -match '(?m)^port:\s*["'']?(\d+)') { [void]$data.ports.Add([int]$Matches[1]) }
        if ($t -match 'socks-port:\s*["'']?(\d+)') { [void]$data.ports.Add([int]$Matches[1]) }
    }
    catch { }
}
$listeningLines = @(netstat.exe -ano 2>$null | Select-String 'LISTENING')
foreach ($l in $listeningLines) {
    $parts = @($l.Line.Trim() -split '\s+')
    if ($parts.Count -ge 5) {
        $pidv = 0
        [void][int]::TryParse($parts[4], [ref]$pidv)
        if ($proxyPids -contains $pidv) {
            [void]$data.listening.Add([ordered]@{ proto = $parts[0]; addr = $parts[1]; state = $parts[3]; pid = $pidv })
            $addr = $parts[1]
            if ($addr -match ':(\d+)$') { [void]$data.ports.Add([int]$Matches[1]) }
        }
    }
}
$ports = @($data.ports | Select-Object -Unique | Select-Object -First 6)
foreach ($p in $ports) {
    foreach ($scheme in @('http', 'socks5')) {
        $proxy = if ($scheme -eq 'http') { "http://127.0.0.1:$p" } else { "socks5://127.0.0.1:$p" }
        $code = (& curl.exe -x $proxy -sS -o NUL -w '%{http_code}' --max-time 15 'https://api.deepseek.com' 2>$null)
        $ex = $LASTEXITCODE
        [void]$data.curl.Add([ordered]@{ scheme = $scheme; port = $p; http = "$code"; exit = $ex })
    }
}
$direct = (& curl.exe -sS -o NUL -w '%{http_code}' --max-time 15 'https://api.deepseek.com' 2>$null)
$data.direct_http = "$direct"
$data.direct_exit = $LASTEXITCODE
$jsonPath = Join-Path $outDir 'netcheck.json'
$data | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $jsonPath -Encoding utf8
Write-Output "NETCHECK_JSON=$jsonPath"
Write-Output "NETCHECK_PROC_COUNT=$($data.processes.Count)"
Write-Output "NETCHECK_PORTS=$($ports -join ',')"
Write-Output 'NETCHECK_DONE'
exit 0
'@

$wrapupPreBody = @'
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
'@

$wrapupApplyBody = @'
$env:Path = [Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [Environment]::GetEnvironmentVariable('Path','User')
$env:PYTHONPATH = 'C:\s4'
$ErrorActionPreference = 'Continue'
$root = 'C:\workspace\wrapup'
$py = 'C:\Program Files\Python312\python.exe'
$hard = 'C:\s4\_windows_high_nist\hardening\apply_hardening.ps1'
$probeRun = 'C:\s4\_windows_high_nist\run\run_enforcement_probe.ps1'
$sandboxCli = 'C:\s4\scripts\run_windows_native_sandbox_command.py'
New-Item -ItemType Directory -Path $root -Force | Out-Null
$summary = [ordered]@{
    ran_at = (Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ')
    apply_exit = -1
    applocker_effective = $false
    applocker_srpv2 = $false
    applocker_exe_rules = 0
    probe1 = $null
    probe2 = $null
    hive_5023_1 = $false
    hive_5023_2 = $false
    hive_mounted_pre_apply = $false
    cred_nonadmin = $null
    cred_highnist = $null
    hive_mounted_post = $false
    hive_locked_post = $false
    errors = New-Object System.Collections.ArrayList
}
# Pre-apply guard: the AgentUser hive must not already be mounted
$u = [ADSI]"WinNT://$env:COMPUTERNAME/AgentUser"
$sidBytes = $u.objectSID[0]
$sid = [System.Security.Principal.SecurityIdentifier]::new($sidBytes, 0).Value
$hkuOut = (& reg.exe query HKU 2>&1) -join "`n"
$summary.hive_mounted_pre_apply = $hkuOut.Contains($sid)
Write-Output "HIVE_PRE_APPLY mounted=$($summary.hive_mounted_pre_apply)"
if ($summary.hive_mounted_pre_apply) {
    [void]$summary.errors.Add('AgentUser hive mounted before apply (leaked profile load)')
}
Write-Output '===== WRAPUP APPLY HIGH-NIST ====='
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File $hard -Arm high-nist -RunUser AgentUser -WorkspaceRoot C:\workspace -DeepSeekIp 221.204.163.76 -EnableAppLocker 2>&1 | ForEach-Object { Write-Output "WRAP_APPLY: $_" }
$summary.apply_exit = $LASTEXITCODE
Write-Output "WRAP_APPLY_EXIT=$($summary.apply_exit)"
if ($summary.apply_exit -ne 0) {
    [void]$summary.errors.Add('apply high-nist failed')
}
# AppLocker re-verify
$alXml = ''
try {
    $alXml = [string](Get-AppLockerPolicy -Effective -Xml -ErrorAction Stop)
}
catch {
    [void]$summary.errors.Add('Get-AppLockerPolicy failed: ' + $_.Exception.Message)
}
try {
    [xml]$x = $alXml
    $exeColl = @($x.AppLockerPolicy.RuleCollection | Where-Object { $_.Type -eq 'Exe' })
    if ($exeColl.Count -gt 0) {
        $summary.applocker_effective = $true
        $summary.applocker_enforcement = [string]$exeColl[0].EnforcementMode
        $summary.applocker_exe_rules = @($exeColl[0].ChildNodes | Where-Object { $_.NodeType -eq 'Element' }).Count
    }
}
catch {
    [void]$summary.errors.Add('AppLocker XML parse failed: ' + $_.Exception.Message)
}
& reg.exe query 'HKLM\SOFTWARE\Policies\Microsoft\Windows\SrpV2' 2>$null | Out-Null
$summary.applocker_srpv2 = ($LASTEXITCODE -eq 0)
Write-Output "APPLOCKER effective=$($summary.applocker_effective) srpv2=$($summary.applocker_srpv2) exe_rules=$($summary.applocker_exe_rules)"
if (-not $summary.applocker_effective -or -not $summary.applocker_srpv2) {
    [void]$summary.errors.Add('AppLocker verify failed')
}
$summary | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $root 'wrapup-apply.json') -Encoding utf8
Write-Output 'WRAPUP_APPLY_DONE'
if ($summary.errors.Count -gt 0) {
    Write-Output "WRAPUP_ERRORS=$($summary.errors.Count)"
    foreach ($e in $summary.errors) { Write-Output "WRAPUP_ERROR: $e" }
    exit 1
}
exit 0
'@

$wrapupProbeBody = @'
$env:Path = [Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [Environment]::GetEnvironmentVariable('Path','User')
$env:PYTHONPATH = 'C:\s4'
$ErrorActionPreference = 'Continue'
$root = 'C:\workspace\wrapup'
$py = 'C:\Program Files\Python312\python.exe'
$probeRun = 'C:\s4\_windows_high_nist\run\run_enforcement_probe.ps1'
$sandboxCli = 'C:\s4\scripts\run_windows_native_sandbox_command.py'
$credExe = 'C:\s4\tools\cred_probe.exe'
New-Item -ItemType Directory -Path $root -Force | Out-Null
$summary = [ordered]@{
    ran_at = (Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ')
    probe1 = $null
    probe2 = $null
    hive_5023_1 = $false
    hive_5023_2 = $false
    cred_nonadmin = $null
    cred_highnist = $null
    hive_mounted_post = $false
    hive_locked_post = $false
    errors = New-Object System.Collections.ArrayList
}
$u = [ADSI]"WinNT://$env:COMPUTERNAME/AgentUser"
$sidBytes = $u.objectSID[0]
$sid = [System.Security.Principal.SecurityIdentifier]::new($sidBytes, 0).Value
# 5023 wrap-up gate: after apply's HKCU-freeze reg load/unload cycle the
# profile service rejects LoadUserProfileW with WinError 5023 for several
# minutes.  Wait (SYSTEM-side, up to 600s) until the AgentUser profile
# loads and unloads cleanly before running the first sandbox probe.
$gateCode = @"
import ctypes, time, sys
from ctypes import wintypes
adv = ctypes.WinDLL('advapi32', use_last_error=True)
ue = ctypes.WinDLL('userenv', use_last_error=True)
adv.LogonUserW.argtypes = [wintypes.LPCWSTR, wintypes.LPCWSTR, wintypes.LPCWSTR, wintypes.DWORD, wintypes.DWORD, ctypes.POINTER(wintypes.HANDLE)]
adv.LogonUserW.restype = wintypes.BOOL
class PI(ctypes.Structure):
    _fields_ = [('dwSize', wintypes.DWORD), ('dwFlags', wintypes.DWORD), ('lpUserName', wintypes.LPWSTR), ('lpProfilePath', wintypes.LPWSTR), ('lpDefaultPath', wintypes.LPWSTR), ('lpServerName', wintypes.LPWSTR), ('lpPolicyPath', wintypes.LPWSTR), ('hProfile', wintypes.HANDLE)]
ue.LoadUserProfileW.argtypes = [wintypes.HANDLE, ctypes.POINTER(PI)]
ue.LoadUserProfileW.restype = wintypes.BOOL
ue.UnloadUserProfile.argtypes = [wintypes.HANDLE, wintypes.HANDLE]
ue.UnloadUserProfile.restype = wintypes.BOOL
tok = wintypes.HANDLE()
ok = adv.LogonUserW('AgentUser', '$env:COMPUTERNAME', 'S4Run!2026', 4, 0, ctypes.byref(tok))
if not ok:
    print('GATE_LOGON_FAIL', ctypes.get_last_error())
    sys.exit(1)
deadline = time.time() + 600
attempt = 0
while time.time() < deadline:
    attempt += 1
    info = PI()
    info.dwSize = ctypes.sizeof(PI)
    info.lpUserName = wintypes.LPWSTR('AgentUser')
    r = ue.LoadUserProfileW(tok, ctypes.byref(info))
    if r:
        ue.UnloadUserProfile(tok, info.hProfile)
        print('GATE_READY attempt', attempt, 'elapsed', round(time.time() - (deadline - 600), 1))
        sys.exit(0)
    err = ctypes.get_last_error()
    if attempt <= 3 or attempt % 5 == 0:
        print('GATE_WAIT attempt', attempt, 'err', err)
    time.sleep(10)
print('GATE_TIMEOUT')
sys.exit(1)
"@
Set-Content -LiteralPath (Join-Path $root 'gate_profile_ready.py') -Value $gateCode -Encoding utf8
Write-Output '===== WRAPUP HIVE READY GATE (5023 cooldown) ====='
& $py (Join-Path $root 'gate_profile_ready.py') 2>&1 | ForEach-Object { Write-Output "GATE: $_" }
$gateExit = $LASTEXITCODE
Write-Output "GATE_EXIT=$gateExit"
if ($gateExit -ne 0) {
    [void]$summary.errors.Add('hive readiness gate timed out (LoadUserProfileW 5023 not clearing)')
}
function Run-WrapupProbe {
    param([string]$Tag)
    $wsDir = Join-Path $root ("probe-ws-$Tag")
    New-Item -ItemType Directory -Path $wsDir -Force | Out-Null
    # ResultPath must live inside the sandbox workdir: the AppContainer
    # child only has write access to the workspace it was granted.
    $resPath = Join-Path $wsDir 'enforcement-probe-high-nist.json'
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $probeRun -Arm high-nist -Workspace $wsDir -ResultPath $resPath -TimeoutSeconds 200 2>&1 | ForEach-Object { Write-Output "PROBE$Tag`: $_" }
    $pExit = $LASTEXITCODE
    if (Test-Path -LiteralPath $resPath) {
        Copy-Item -LiteralPath $resPath -Destination (Join-Path $root ("enforcement-probe-wrapup-$Tag.json")) -Force
    }
    $obsSrc = Join-Path $wsDir 'windows-native-run-observation-high-nist.json'
    $obsCopy = Join-Path $root ("windows-native-run-observation-wrapup-$Tag.json")
    if (Test-Path -LiteralPath $obsSrc) {
        Copy-Item -LiteralPath $obsSrc -Destination $obsCopy -Force
    }
    $hiveHit = $false
    if (Test-Path -LiteralPath $obsSrc) {
        $o = Get-Content -LiteralPath $obsSrc -Raw -Encoding UTF8 | ConvertFrom-Json
        if ($o.diagnostics) {
            $diagShown = 0
            foreach ($d in $o.diagnostics) {
                if ($d -match '5023|LoadUserProfileW.*failed') { $hiveHit = $true }
                if ($diagShown -lt 25) {
                    Write-Output "PROBE${Tag}_DIAG: $d"
                    $diagShown++
                }
            }
        }
    }
    return [ordered]@{ tag = $Tag; probe_exit = $pExit; obs_copied = (Test-Path -LiteralPath $obsCopy); hive_5023 = $hiveHit }
}
Write-Output '===== WRAPUP PROBE 1 (immediate after apply) ====='
$summary.probe1 = Run-WrapupProbe -Tag '1'
$summary.hive_5023_1 = [bool]$summary.probe1.hive_5023
Write-Output "PROBE1 exit=$($summary.probe1.probe_exit) hive5023=$($summary.hive_5023_1)"
Write-Output '===== WRAPUP PROBE 2 (stability) ====='
$summary.probe2 = Run-WrapupProbe -Tag '2'
$summary.hive_5023_2 = [bool]$summary.probe2.hive_5023
Write-Output "PROBE2 exit=$($summary.probe2.probe_exit) hive5023=$($summary.hive_5023_2)"
if ($summary.probe1.probe_exit -ne 0 -or $summary.probe2.probe_exit -ne 0) {
    [void]$summary.errors.Add('enforcement probe failed after apply')
}
# 5023 wrap-up note: probes stay compliant (exit 0, wall assertions pass)
# even when the sandbox-side LoadUserProfileW logs a 5023 diagnostic; the
# profile-service state behind it is registered as a known gap (real HKCU
# mapping needs a dedicated SYSTEM job per sandbox run or a kept-loaded
# profile).  Recorded, not fatal.
$summary.hive_5023_note = ($summary.hive_5023_1 -or $summary.hive_5023_2)
if ($summary.hive_5023_note) {
    Write-Output 'HIVE_5023_NOTE=present (compliant; registered gap)'
}
# CredReadW inside sandbox with the NATIVE cred_probe.exe (python ctypes is
# not usable inside an AppContainer child - _ctypes/libffi DLL init fails;
# the native exe mirrors orz's own CredReadW path).
$credStates = @(
    @{ key = 'nonadmin'; arm = 'non-admin' },
    @{ key = 'highnist'; arm = 'high-nist' }
)
foreach ($cs in $credStates) {
    $wd = Join-Path $root ("cred-" + $cs.key)
    New-Item -ItemType Directory -Path $wd -Force | Out-Null
    $wdExe = Join-Path $wd 'cred_probe.exe'
    Copy-Item -LiteralPath $credExe -Destination $wdExe -Force
    if (-not (Test-Path -LiteralPath $wdExe)) {
        [void]$summary.errors.Add("cred_probe.exe missing for $($cs.key)")
        continue
    }
    $marker = Join-Path $wd '.assurance-p2-disposable.json'
    @{
        schema_version = '0.1.0-draft'
        purpose = 'windows-native-sandbox-probe'
        allow_container_write_probe = $true
    } | ConvertTo-Json | Set-Content -LiteralPath $marker -Encoding ascii
    $obs = Join-Path $root ("windows-native-run-observation-cred-" + $cs.key + '.json')
    $resIn = Join-Path $wd 'cred-result.json'
    $resOut = Join-Path $root ("cred-" + $cs.key + '.json')
    $sbxArgs = @(
        '--workspace', $wd,
        '--arm', $cs.arm,
        '--timeout', 240,
        '--output', $obs,
        '--command',
        $wdExe, $resIn
    )
    Write-Output "===== CRED SANDBOX arm=$($cs.arm) ====="
    & $py $sandboxCli @sbxArgs 2>&1 | ForEach-Object { Write-Output "CREDSBX: $_" }
    $sbxExit = $LASTEXITCODE
    Write-Output "CRED_SBX_EXIT=$sbxExit"
    if (Test-Path -LiteralPath $resIn) {
        Copy-Item -LiteralPath $resIn -Destination $resOut -Force
        $credRes = Get-Content -LiteralPath $resOut -Raw -Encoding UTF8 | ConvertFrom-Json
        if ($cs.key -eq 'nonadmin') { $summary.cred_nonadmin = $credRes }
        else { $summary.cred_highnist = $credRes }
        Write-Output "CRED_$($cs.key)_OK=$($credRes.ok)"
        if (-not $credRes.ok) {
            # Expected friction (design #8): an AppContainer token with
            # empty capabilities cannot read the user Credential Manager
            # store (CredReadW -> WinError 5).  Record it; only unexpected
            # failures fail the stage.
            if ($cs.key -eq 'highnist' -and [int]$credRes.winerror -eq 5) {
                $summary.cred_highnist_denied = $true
                Write-Output 'CRED_HIGHNIST_DENIED_EXPECTED=1'
            }
            else {
                [void]$summary.errors.Add("cred sandbox $($cs.key) read failed (winerror=$($credRes.winerror))")
            }
        }
    }
    else {
        [void]$summary.errors.Add("cred sandbox $($cs.key) result missing (sbxExit=$sbxExit)")
    }
}
# Post-wrapup hive check: every sandbox run must have unloaded the
# AgentUser profile hive before the stage ends.
$hkuOutPost = (& reg.exe query HKU 2>&1) -join "`n"
$summary.hive_mounted_post = $hkuOutPost.Contains($sid)
$hiveFilePost = 'C:\Users\AgentUser\NTUSER.DAT'
try {
    $fs = [System.IO.File]::Open($hiveFilePost, 'Open', 'ReadWrite', 'None')
    $fs.Close()
    $fs.Dispose()
    $summary.hive_locked_post = $false
}
catch {
    $summary.hive_locked_post = $true
    [void]$summary.errors.Add("NTUSER.DAT still locked after wrapup: $($_.Exception.Message)")
}
Write-Output "HIVE_POST mounted=$($summary.hive_mounted_post) locked=$($summary.hive_locked_post)"
$summary | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $root 'wrapup-summary.json') -Encoding utf8
Write-Output 'WRAPUP_MAIN_DONE'
if ($summary.errors.Count -gt 0) {
    Write-Output "WRAPUP_ERRORS=$($summary.errors.Count)"
    foreach ($e in $summary.errors) { Write-Output "WRAPUP_ERROR: $e" }
    exit 1
}
exit 0
'@

# --- dispatch ----------------------------------------------------------------

try {
    switch ($Stage) {
        'restore' {
            Write-Both "STAGE=restore"
            $vm = Get-VM -Name $vmName -ErrorAction Stop
            Write-Both "STATE_BEFORE=$($vm.State)"
            if ($vm.State -ne 'Off' -and $vm.State -ne 'Saved') {
                Write-Both 'ACTION=stop-vm'
                Stop-VM -Name $vmName -ErrorAction Stop
                $deadline = (Get-Date).AddMinutes(3)
                while ((Get-Date) -lt $deadline) {
                    Start-Sleep -Seconds 5
                    $cur = Get-VM -Name $vmName -ErrorAction SilentlyContinue
                    if ($null -ne $cur -and ($cur.State -eq 'Off' -or $cur.State -eq 'Saved')) {
                        break
                    }
                }
                $vm = Get-VM -Name $vmName
                Write-Both "STATE_AFTER_STOP=$($vm.State)"
                if ($vm.State -ne 'Off' -and $vm.State -ne 'Saved') {
                    throw 'VM did not stop'
                }
            }
            $snap = Get-VMSnapshot -VMName $vmName -Name $checkpoint -ErrorAction Stop
            if (-not $snap) {
                throw "baseline checkpoint $checkpoint missing"
            }
            Write-Both "CHECKPOINT=$($snap.Name) created=$($snap.CreationTime)"
            Restore-VMCheckpoint -VMName $vmName -Name $checkpoint -Confirm:$false -ErrorAction Stop
            Write-Both 'RESTORE_OK=1'
            Start-VM -Name $vmName -ErrorAction Stop
            Write-Both 'START_ISSUED=1'
            Wait-VmReachable
        }
        'sync' {
            Write-Both "STAGE=sync"
            Sync-Harness
        }
        'setup' {
            Write-Both "STAGE=setup"
            $code = Invoke-GuestJob -Name 'formal-setup' -Body $setupBody -TimeoutSeconds 300
            if ($code -ne 0) {
                throw "formal setup job failed (exit $code)"
            }
        }
        'control' {
            Write-Both "STAGE=control"
            $code = Invoke-GuestJob -Name 'formal-control' -Body $controlBody -TimeoutSeconds 400
            if ($code -ne 0) {
                throw "control probe failed (exit $code) - arm not valid (fail-closed)"
            }
            Collect-VmEvidence -Arm 'control'
        }
        'taskcontrol' {
            Invoke-TaskArmStage -StageName 'taskcontrol' `
                -JobName 'task-baseline-control' -Body $taskcontrolBody `
                -JobTimeoutSeconds 1200 -ArmLabel 'task-control' `
                -GuestWorkspace 'C:\workspace\taskcontrol' `
                -SnapshotName $checkpoint
        }
        'tasknonadmin' {
            Invoke-TaskArmStage -StageName 'tasknonadmin' `
                -JobName 'task-baseline-non-admin' -Body $tasknonadminBody `
                -JobTimeoutSeconds 1800 -ArmLabel 'task-non-admin' `
                -GuestWorkspace 'C:\workspace\tasknonadmin' `
                -SnapshotName $checkpoint
        }
        'taskhighnist' {
            Invoke-TaskArmStage -StageName 'taskhighnist' `
                -JobName 'task-baseline-high-nist' -Body $taskhighnistBody `
                -JobTimeoutSeconds 1800 -ArmLabel 'task-high-nist' `
                -GuestWorkspace 'C:\workspace\taskhighnist' `
                -SnapshotName $checkpoint
        }
        'agentcontrol' {
            Invoke-TaskArmStage -StageName 'agentcontrol' `
                -JobName 'agent-baseline-control' -Body $agentcontrolBody `
                -JobTimeoutSeconds 10800 -ArmLabel 'agent-control' `
                -GuestWorkspace 'C:\workspace\agentcontrol' `
                -SnapshotName $checkpoint
        }
        'agenthighnist' {
            Invoke-TaskArmStage -StageName 'agenthighnist' `
                -JobName 'agent-baseline-high-nist' -Body $agenthighnistBody `
                -JobTimeoutSeconds 10800 -ArmLabel 'agent-high-nist' `
                -GuestWorkspace 'C:\workspace\agenthighnist' `
                -SnapshotName $checkpoint
        }
        'netcheck' {
            Write-Both "STAGE=netcheck"
            $code = Invoke-GuestJob -Name 'formal-netcheck' -Body $netcheckBody -TimeoutSeconds 300
            if ($code -ne 0) {
                throw "netcheck job failed (exit $code)"
            }
            Collect-TaskEvidence -ArmLabel 'netcheck' `
                -GuestWorkspace 'C:\workspace\netcheck' `
                -NameFilter '^netcheck.*\.json$'
        }
        'wrapup' {
            Write-Both "STAGE=wrapup"
            $vm = Get-VM -Name $vmName -ErrorAction Stop
            Write-Both "STATE_BEFORE=$($vm.State)"
            if ($vm.State -ne 'Off' -and $vm.State -ne 'Saved') {
                Write-Both 'ACTION=stop-vm'
                Stop-VM -Name $vmName -ErrorAction Stop
                $deadline = (Get-Date).AddMinutes(3)
                while ((Get-Date) -lt $deadline) {
                    Start-Sleep -Seconds 5
                    $cur = Get-VM -Name $vmName -ErrorAction SilentlyContinue
                    if ($null -ne $cur -and ($cur.State -eq 'Off' -or $cur.State -eq 'Saved')) {
                        break
                    }
                }
                $vm = Get-VM -Name $vmName
                Write-Both "STATE_AFTER_STOP=$($vm.State)"
                if ($vm.State -ne 'Off' -and $vm.State -ne 'Saved') {
                    throw 'VM did not stop'
                }
            }
            $snap = Get-VMSnapshot -VMName $vmName -Name $checkpoint -ErrorAction Stop
            if (-not $snap) {
                throw "baseline checkpoint $checkpoint missing"
            }
            Write-Both "CHECKPOINT=$($snap.Name) created=$($snap.CreationTime)"
            Restore-VMCheckpoint -VMName $vmName -Name $checkpoint -Confirm:$false -ErrorAction Stop
            Write-Both 'RESTORE_OK=1'
            Start-VM -Name $vmName -ErrorAction Stop
            Write-Both 'START_ISSUED=1'
            Wait-VmReachable
            Sync-Harness
            $code = Invoke-GuestJob -Name 'formal-setup' -Body $setupBody -TimeoutSeconds 300
            if ($code -ne 0) {
                throw "formal setup job failed (exit $code)"
            }
            $code = Invoke-GuestJob -Name 'wrapup-pre' -Body $wrapupPreBody -TimeoutSeconds 900
            if ($code -ne 0) {
                throw "wrapup-pre job failed (exit $code)"
            }
            $code = Invoke-GuestJob -Name 'wrapup-apply' -Body $wrapupApplyBody -TimeoutSeconds 1200
            if ($code -ne 0) {
                throw "wrapup-apply job failed (exit $code)"
            }
            $code = Invoke-GuestJob -Name 'wrapup-probe' -Body $wrapupProbeBody -TimeoutSeconds 2400
            if ($code -ne 0) {
                throw "wrapup-probe job failed (exit $code)"
            }
            Collect-TaskEvidence -ArmLabel 'wrapup' `
                -GuestWorkspace 'C:\workspace\wrapup' `
                -NameFilter '^(wrapup-|enforcement-probe-|windows-native-run-observation-|cred-)'
        }
        'nonadmin' {
            Write-Both "STAGE=nonadmin"
            $code = Invoke-GuestJob -Name 'formal-nonadmin' -Body $nonadminBody -TimeoutSeconds 600
            if ($code -ne 0) {
                throw "non-admin probe failed (exit $code) - arm not valid (fail-closed)"
            }
            Collect-VmEvidence -Arm 'non-admin'
        }
        'highnist' {
            Write-Both "STAGE=highnist"
            Sync-Harness
            $code = Invoke-GuestJob -Name 'formal-highnist' -Body $highnistBody -TimeoutSeconds 700
            if ($code -ne 0) {
                throw "high-nist probe failed (exit $code) - arm not valid (fail-closed)"
            }
            Collect-VmEvidence -Arm 'high-nist'
        }
        'reboot' {
            Write-Both "STAGE=reboot"
            $vm = Get-VM -Name $vmName -ErrorAction Stop
            Write-Both "STATE_BEFORE=$($vm.State)"
            if ($vm.State -eq 'Running') {
                Write-Both 'ACTION=stop-vm-graceful'
                Stop-VM -Name $vmName -ErrorAction Stop
                $deadline = (Get-Date).AddMinutes(3)
                while ((Get-Date) -lt $deadline) {
                    Start-Sleep -Seconds 5
                    $cur = Get-VM -Name $vmName -ErrorAction SilentlyContinue
                    if ($null -ne $cur -and ($cur.State -eq 'Off' -or $cur.State -eq 'Saved')) {
                        break
                    }
                }
                $vm = Get-VM -Name $vmName
                Write-Both "STATE_AFTER_STOP=$($vm.State)"
                if ($vm.State -ne 'Off' -and $vm.State -ne 'Saved') {
                    throw 'VM did not stop'
                }
            }
            Start-VM -Name $vmName -ErrorAction Stop
            Write-Both 'START_ISSUED=1'
            Wait-VmReachable
        }
        'all' {
            Write-Both "STAGE=all"
            Sync-Harness
            $code = Invoke-GuestJob -Name 'formal-setup' -Body $setupBody -TimeoutSeconds 300
            if ($code -ne 0) {
                throw "formal setup job failed (exit $code)"
            }
            $code = Invoke-GuestJob -Name 'formal-control' -Body $controlBody -TimeoutSeconds 400
            if ($code -ne 0) {
                throw "control probe failed (exit $code) - arm not valid (fail-closed)"
            }
            Collect-VmEvidence -Arm 'control'
            $code = Invoke-GuestJob -Name 'formal-nonadmin' -Body $nonadminBody -TimeoutSeconds 600
            if ($code -ne 0) {
                throw "non-admin probe failed (exit $code) - arm not valid (fail-closed)"
            }
            Collect-VmEvidence -Arm 'non-admin'
            $code = Invoke-GuestJob -Name 'formal-highnist' -Body $highnistBody -TimeoutSeconds 700
            if ($code -ne 0) {
                throw "high-nist probe failed (exit $code) - arm not valid (fail-closed)"
            }
            Collect-VmEvidence -Arm 'high-nist'
        }
        'full' {
            Write-Both "STAGE=full"
            $vm = Get-VM -Name $vmName -ErrorAction Stop
            Write-Both "STATE_BEFORE=$($vm.State)"
            if ($vm.State -ne 'Off' -and $vm.State -ne 'Saved') {
                Write-Both 'ACTION=stop-vm'
                Stop-VM -Name $vmName -ErrorAction Stop
                $deadline = (Get-Date).AddMinutes(3)
                while ((Get-Date) -lt $deadline) {
                    Start-Sleep -Seconds 5
                    $cur = Get-VM -Name $vmName -ErrorAction SilentlyContinue
                    if ($null -ne $cur -and ($cur.State -eq 'Off' -or $cur.State -eq 'Saved')) {
                        break
                    }
                }
                $vm = Get-VM -Name $vmName
                Write-Both "STATE_AFTER_STOP=$($vm.State)"
                if ($vm.State -ne 'Off' -and $vm.State -ne 'Saved') {
                    throw 'VM did not stop'
                }
            }
            $snap = Get-VMSnapshot -VMName $vmName -Name $checkpoint -ErrorAction Stop
            if (-not $snap) {
                throw "baseline checkpoint $checkpoint missing"
            }
            Write-Both "CHECKPOINT=$($snap.Name) created=$($snap.CreationTime)"
            Restore-VMCheckpoint -VMName $vmName -Name $checkpoint -Confirm:$false -ErrorAction Stop
            Write-Both 'RESTORE_OK=1'
            Start-VM -Name $vmName -ErrorAction Stop
            Write-Both 'START_ISSUED=1'
            Wait-VmReachable
            Sync-Harness
            $code = Invoke-GuestJob -Name 'formal-setup' -Body $setupBody -TimeoutSeconds 300
            if ($code -ne 0) {
                throw "formal setup job failed (exit $code)"
            }
            $code = Invoke-GuestJob -Name 'formal-control' -Body $controlBody -TimeoutSeconds 400
            if ($code -ne 0) {
                throw "control probe failed (exit $code) - arm not valid (fail-closed)"
            }
            Collect-VmEvidence -Arm 'control'
            $code = Invoke-GuestJob -Name 'formal-nonadmin' -Body $nonadminBody -TimeoutSeconds 600
            if ($code -ne 0) {
                throw "non-admin probe failed (exit $code) - arm not valid (fail-closed)"
            }
            Collect-VmEvidence -Arm 'non-admin'
            $code = Invoke-GuestJob -Name 'formal-highnist' -Body $highnistBody -TimeoutSeconds 700
            if ($code -ne 0) {
                throw "high-nist probe failed (exit $code) - arm not valid (fail-closed)"
            }
            Collect-VmEvidence -Arm 'high-nist'
        }
    }
}
catch {
    Write-Exit "ERROR: $($_.Exception.Message)"
    Write-Exit "STAGE_ABORTED=$Stage"
    exit 1
}

Write-Exit "STAGE_DONE=$Stage"
exit 0
