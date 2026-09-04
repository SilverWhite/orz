<#
    S4 live agent-round runner (host-side elevated, bridge op vm-agent).

    Runs run_agent_arm.ps1 inside win-s4 on the CURRENT live VM state (no
    checkpoint restore: the live high-nist wall + injected orz.exe + AgentUser
    credential are the intended agent baseline, 2026-09-03 ruling).

      -Arm high-nist|control   target arm (default high-nist)
      -TaskIds a,b,c           tasks from the selected task set (CSV)
      -TaskSet tb2.1|friction  task tree on the guest
      -RunTag <tag>            evidence dir + workspace suffix
      -DryRun                  print-only plan; no credential read, no spawn

    Steps: validate ids against the host-side task set -> sync runner / tools /
    task tree / orz.exe into the VM -> run one elevated guest job that copies
    orz.exe into Program Files and invokes run_agent_arm.ps1 -> collect
    evidence into <OutDir>/evidence-agent-<RunTag>-<Arm>/.
    ASCII only.
#>
[CmdletBinding()]
param(
    [ValidateSet('control', 'high-nist')]
    [string]$Arm = 'high-nist',

    [Parameter(Mandatory = $true)]
    [string]$TaskIds,

    [ValidateSet('tb2.1', 'friction')]
    [string]$TaskSet = 'tb2.1',

    [string]$RunTag = '',

    [string]$KeyFile = '',

    [string]$OutDir = 'D:\CLI\_windows_high_nist\formal-2026-09-02',

    [switch]$DryRun
)

$ErrorActionPreference = 'Stop'
$vmName = 'win-s4'
$credUser = 'HL'
$credPass = '123456'
$runner = 'D:\CLI\scripts\s4_vm_run_elev.ps1'
$lines = New-Object System.Collections.Generic.List[string]

$taskSetDir = 'D:\CLI\_windows_high_nist\agent-tasks-tb2.1'
$guestSetRoot = 'C:\s4\_windows_high_nist\agent-tasks-tb2.1'
if ($TaskSet -eq 'friction') {
    $taskSetDir = 'D:\CLI\_windows_high_nist\tasks'
    $guestSetRoot = 'C:\s4\_windows_high_nist\tasks'
}

$ids = @($TaskIds -split ',' | ForEach-Object { $_.Trim() } | Where-Object { $_ })
if ($ids.Count -eq 0) {
    throw 'TaskIds is empty'
}
if (-not $RunTag) {
    $RunTag = $ids[0]
}
$RunTag = ($RunTag -replace '[^A-Za-z0-9._-]', '_')
$runLabel = "$TaskSet-$RunTag"
$wsGuest = "C:\workspace\agent-$runLabel"

# ---- validate ids + estimate guest-job wallclock ---------------------------
$manifestPath = Join-Path $taskSetDir 'manifest.json'
if (-not (Test-Path -LiteralPath $manifestPath)) {
    throw "task set manifest missing: $manifestPath"
}
$m = Get-Content -LiteralPath $manifestPath -Raw -Encoding UTF8 | ConvertFrom-Json
$knownIds = @($m.tasks | ForEach-Object { $_.id })
$estSeconds = 900
foreach ($id in $ids) {
    if ($knownIds -notcontains $id) {
        throw "unknown task id in $TaskSet set: $id"
    }
    $tjPath = Join-Path $taskSetDir "$id\task.json"
    $to = 960
    if (Test-Path -LiteralPath $tjPath) {
        $tj = Get-Content -LiteralPath $tjPath -Raw -Encoding UTF8 | ConvertFrom-Json
        if ($tj.agent_timeout_seconds) {
            $to = [int]$tj.agent_timeout_seconds
        }
    }
    $estSeconds += $to + 180
}
if ($estSeconds -gt 14000) {
    $estSeconds = 14000
}
$jobTimeout = if ($DryRun) { 600 } else { $estSeconds }
$lines.Add("AGENT_PLAN arm=$Arm taskSet=$TaskSet ids=$($ids -join ',') runTag=$RunTag")
$lines.Add("AGENT_PLAN workspace=$wsGuest jobTimeout=$jobTimeout dryRun=$DryRun")

# ---- sync harness into the live VM ----------------------------------------
$sec = ConvertTo-SecureString $credPass -AsPlainText -Force
$cred = New-Object System.Management.Automation.PSCredential($credUser, $sec)
$sess = $null
$deadline = (Get-Date).AddSeconds(300)
while (-not $sess -and (Get-Date) -lt $deadline) {
    try {
        $sess = New-PSSession -VMName $vmName -Credential $cred -ErrorAction Stop
    }
    catch {
        Start-Sleep -Seconds 10
    }
}
if (-not $sess) {
    throw "win-s4 not reachable within 300s"
}
try {
    $guestDirs = @(
        'C:\s4\assurance',
        'C:\s4\tools',
        'C:\s4\scripts',
        'C:\s4\_windows_high_nist\run',
        'C:\s4\_windows_high_nist\agent-tasks-tb2.1',
        'C:\s4\_windows_high_nist\tasks'
    )
    Invoke-Command -Session $sess -ArgumentList $guestDirs -ScriptBlock {
        param($dirs)
        foreach ($d in $dirs) {
            New-Item -ItemType Directory -Path $d -Force | Out-Null
        }
    }

    $syncFiles = @(
        @{ Src = 'D:\CLI\_windows_high_nist\run\run_agent_arm.ps1'; Dst = 'C:\s4\_windows_high_nist\run\run_agent_arm.ps1' },
        @{ Src = 'D:\CLI\assurance\windows_sandbox.py'; Dst = 'C:\s4\assurance\windows_sandbox.py' },
        @{ Src = 'D:\CLI\assurance\sandbox_verifier.py'; Dst = 'C:\s4\assurance\sandbox_verifier.py' },
        @{ Src = 'D:\CLI\assurance\windows-native-sandbox-run-v0.1.schema.json'; Dst = 'C:\s4\assurance\windows-native-sandbox-run-v0.1.schema.json' },
        @{ Src = 'D:\CLI\_windows_high_nist\tools\cred_probe.exe'; Dst = 'C:\s4\tools\cred_probe.exe' },
        @{ Src = 'D:\CLI\scripts\run_windows_native_sandbox_command.py'; Dst = 'C:\s4\scripts\run_windows_native_sandbox_command.py' },
        @{ Src = 'D:\CLI\orz\target\release\orz.exe'; Dst = 'C:\s4\tools\orz.exe' },
        @{ Src = 'D:\CLI\orz\target\release\orz-signer.exe'; Dst = 'C:\s4\tools\orz-signer.exe' },
        @{ Src = 'D:\CLI\orz\target\release\orz-acaf-provision.exe'; Dst = 'C:\s4\tools\orz-acaf-provision.exe' }
    )
    foreach ($t in $syncFiles) {
        Copy-Item -LiteralPath $t.Src -Destination $t.Dst -ToSession $sess -Force
        $srcHash = (Get-FileHash -LiteralPath $t.Src -Algorithm SHA256).Hash
        $dstHash = Invoke-Command -Session $sess -ArgumentList $t.Dst -ScriptBlock {
            param($p) (Get-FileHash -LiteralPath $p -Algorithm SHA256).Hash
        }
        $ok = ($srcHash -eq $dstHash)
        $lines.Add("SYNC $($t.Dst) ok=$ok")
        if (-not $ok) {
            throw "sync hash mismatch: $($t.Dst)"
        }
    }

    Copy-Item -LiteralPath $taskSetDir -Destination 'C:\s4\_windows_high_nist' -Recurse -Force -ToSession $sess
    $lines.Add("SYNC_TASKSET $TaskSet -> $guestSetRoot ok")
    if ($KeyFile) {
        if (-not (Test-Path -LiteralPath $KeyFile)) {
            throw "key file missing: $KeyFile"
        }
        $guestKeyStage = 'C:\s4\tools\ds-key-stage.txt'
        Copy-Item -LiteralPath $KeyFile -Destination $guestKeyStage -ToSession $sess -Force
        $kh1 = (Get-FileHash -LiteralPath $KeyFile -Algorithm SHA256).Hash
        $kh2 = Invoke-Command -Session $sess -ArgumentList $guestKeyStage -ScriptBlock {
            param($p) (Get-FileHash -LiteralPath $p -Algorithm SHA256).Hash
        }
        $keyOk = ($kh1 -eq $kh2)
        $lines.Add("SYNC_KEY_STAGE ok=$keyOk")
        if (-not $keyOk) {
            throw "key stage hash mismatch: $guestKeyStage"
        }
    }
}
finally {
    Remove-PSSession -Session $sess -ErrorAction SilentlyContinue
}

# ---- guest job body --------------------------------------------------------
$dryFlag = ''
if ($DryRun) {
    $dryFlag = ' -DryRun'
}
$body = @'
$env:Path = [Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [Environment]::GetEnvironmentVariable('Path','User')
$env:PYTHONPATH = 'C:\s4'
foreach ($f in @('orz.exe', 'orz-signer.exe', 'orz-acaf-provision.exe')) {
    $orzSrc = Join-Path 'C:\s4\tools' $f
    $orzDst = Join-Path 'C:\Program Files\orz' $f
    if (Test-Path -LiteralPath $orzSrc) {
        Copy-Item -LiteralPath $orzSrc -Destination $orzDst -Force
        $h1 = (Get-FileHash -LiteralPath $orzSrc -Algorithm SHA256).Hash
        $h2 = (Get-FileHash -LiteralPath $orzDst -Algorithm SHA256).Hash
        Write-Output "AGENT_ORZ_SYNC $f ok=$($h1 -eq $h2)"
    }
}
$agentRun = 'C:\s4\_windows_high_nist\run\run_agent_arm.ps1'
$ws = '__WORKSPACE__'
$setDir = '__SETDIR__'
$ids = '__IDS__'
$arm = '__ARM__'
# F9 (2026-09-03): clean stale cross-batch agent workspaces under C:\workspace
# so later runs cannot read prior runs' journals/results (keep C:\workspace\acaf
# and the current run's own workspace).  Evidence lives host-side already.
$agentRoot = 'C:\workspace'
Get-ChildItem -LiteralPath $agentRoot -Directory -Filter 'agent-*' -ErrorAction SilentlyContinue |
    Where-Object { $_.FullName -ne $ws } |
    ForEach-Object {
        Remove-Item -LiteralPath $_.FullName -Recurse -Force -ErrorAction SilentlyContinue
        Write-Output "AGENT_WS_CLEAN removed=$($_.FullName)"
    }
if (Test-Path -LiteralPath $ws) {
    Remove-Item -LiteralPath $ws -Recurse -Force -ErrorAction SilentlyContinue
}
New-Item -ItemType Directory -Path $ws -Force | Out-Null
$keyStage = 'C:\s4\tools\ds-key-stage.txt'
$keyOverride = 'C:\workspace\agent-key.txt'
if (Test-Path -LiteralPath $keyStage) {
    Copy-Item -LiteralPath $keyStage -Destination $keyOverride -Force
    $env:ORZ_AGENT_KEY_FILE = $keyOverride
    Write-Output 'AGENT_KEY_OVERRIDE_STAGED=1'
}
Write-Output "===== AGENT LIVE RUN arm=$arm taskSet=$setDir ids=$ids dryRun=__DRY__ ====="
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File $agentRun -Arm $arm -Workspace $ws `
    -TasksDir $setDir -TaskIds $ids -ResultPath (Join-Path $ws "agent-baseline-$arm.json") __DRYFLAG__ 2>&1 |
    ForEach-Object { Write-Output "AGENTLIVE: $_" }
$code = $LASTEXITCODE
Write-Output "AGENT_LIVE_EXIT=$code"
if (Test-Path -LiteralPath (Join-Path $ws "agent-baseline-$arm.json")) {
    Get-Content -LiteralPath (Join-Path $ws "agent-baseline-$arm.json") -Raw
}
foreach ($p in @($keyOverride, $keyStage)) {
    if (Test-Path -LiteralPath $p) {
        Remove-Item -LiteralPath $p -Force -ErrorAction SilentlyContinue
    }
}
Write-Output 'AGENT_KEY_SCRUB=1'
exit $code
'@
$body = $body.Replace('__WORKSPACE__', $wsGuest)
$body = $body.Replace('__SETDIR__', $guestSetRoot)
$body = $body.Replace('__IDS__', ($ids -join ','))
$body = $body.Replace('__ARM__', $Arm)
$body = $body.Replace('__DRY__', "$DryRun")
$body = $body.Replace('__DRYFLAG__', $dryFlag.Trim())

$bodyFile = Join-Path $OutDir "job-agent-live-$runLabel-body.ps1"
$resFile = Join-Path $OutDir "job-agent-live-$runLabel-result.txt"
New-Item -ItemType Directory -Path $OutDir -Force | Out-Null
Set-Content -LiteralPath $bodyFile -Value $body -Encoding utf8
Remove-Item -LiteralPath $resFile -Force -ErrorAction SilentlyContinue
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File $runner `
    -JobBodyFile $bodyFile -OutFile $resFile -TimeoutSeconds $jobTimeout
$runnerExit = $LASTEXITCODE
$lines.Add("GUEST_RUNNER_EXIT=$runnerExit")
if (Test-Path -LiteralPath $resFile) {
    foreach ($l in (Get-Content -LiteralPath $resFile -Encoding UTF8)) {
        $lines.Add($l)
    }
}

# ---- evidence collect ------------------------------------------------------
$evDir = Join-Path $OutDir "evidence-agent-$runLabel-$Arm"
New-Item -ItemType Directory -Path $evDir -Force | Out-Null
$sess2 = New-PSSession -VMName $vmName -Credential $cred -ErrorAction Stop
try {
    $names = @(Invoke-Command -Session $sess2 -ArgumentList $wsGuest -ScriptBlock {
        param($p)
        $pat = '^(agent-|cred-bootstrap-|journal-|obs-|run-observation-|enforcement-probe-|windows-native-run-observation-)'
        $found = @(Get-ChildItem -LiteralPath $p -Filter '*.json' -File -ErrorAction SilentlyContinue |
            Select-Object -ExpandProperty Name | Where-Object { $_ -match $pat })
        $found += @(Get-ChildItem -LiteralPath $p -Filter '*.jsonl' -File -ErrorAction SilentlyContinue |
            Select-Object -ExpandProperty Name | Where-Object { $_ -match $pat })
        $found
    })
    foreach ($n in $names) {
        Copy-Item -LiteralPath (Join-Path $wsGuest $n) -Destination (Join-Path $evDir $n) -FromSession $sess2 -Force
        $lines.Add("EVIDENCE_COPIED=$(Join-Path $evDir $n)")
    }
    if ($names.Count -eq 0) {
        $lines.Add('EVIDENCE_AGENT_NONE')
    }
}
finally {
    Remove-PSSession -Session $sess2 -ErrorAction SilentlyContinue
}

$ok = ($runnerExit -eq 0)
$lines.Add("AGENT_RUN_OK=$ok")
$lines | Set-Content -LiteralPath (Join-Path $OutDir "vm-agent-run-$RunTag.txt") -Encoding utf8
if (-not $ok) {
    exit 1
}
exit 0
