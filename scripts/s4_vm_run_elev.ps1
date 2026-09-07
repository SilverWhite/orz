<#
    S4 generic in-VM elevated job runner (host-side elevated).
    Runs $JobBody (PowerShell, ASCII) inside win-s4 with a full admin
    (high-IL) token via a scheduled task (RunLevel=Highest), without any
    guest-side UAC prompt. Captures output + exit code to $OutFile.
    ASCII only.
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$JobBodyFile,

    [string]$OutFile = 'D:\CLI\_windows_high_nist\vm-elevjob-result.txt',

    [int]$TimeoutSeconds = 900
)

$ErrorActionPreference = 'Stop'
$lines = New-Object System.Collections.Generic.List[string]
$failed = $false

try {
    if (-not (Test-Path -LiteralPath $JobBodyFile)) {
        throw "JobBodyFile not found: $JobBodyFile"
    }
    $JobBody = Get-Content -LiteralPath $JobBodyFile -Raw -Encoding UTF8
    $sec = ConvertTo-SecureString '123456' -AsPlainText -Force
    $cred = New-Object System.Management.Automation.PSCredential('HL', $sec)
    $sess = New-PSSession -VMName 'win-s4' -Credential $cred

    $bodyPath = 'C:\s4\tools\elev-body.ps1'
    $runnerPath = 'C:\s4\tools\elev-runner.ps1'
    $runId = ([guid]::NewGuid().ToString('N').Substring(0, 12))
    $taskName = 's4elevjob_' + $runId
    $logPath = 'C:\s4\tools\elev-job-' + $runId + '.log'

    $runner = @'
$log = 'C:\s4\tools\elev-job-__RUNID__.log'
Remove-Item -LiteralPath $log -Force -ErrorAction SilentlyContinue
try {
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File 'C:\s4\tools\elev-body.ps1' *> $log
    $code = $LASTEXITCODE
    if ($null -eq $code) { $code = 0 }
} catch {
    "ERROR=$($_.Exception.Message)" | Out-File -Encoding utf8 $log
    $code = 1
}
"EXIT=$code" | Add-Content -LiteralPath $log -Encoding utf8
'@
    $runner = $runner.Replace('__RUNID__', $runId)

    Invoke-Command -Session $sess -ScriptBlock {
        New-Item -ItemType Directory -Path 'C:\s4\tools' -Force | Out-Null
    }
    $bodyBytes = [System.Text.Encoding]::UTF8.GetBytes($JobBody)
    $b64 = [Convert]::ToBase64String($bodyBytes)
    Invoke-Command -Session $sess -ArgumentList $b64, $bodyPath, $runner, $runnerPath -ScriptBlock {
        param($B64, $BodyPath, $Runner, $RunnerPath)
        $scriptText = [System.Text.Encoding]::UTF8.GetString([Convert]::FromBase64String($B64))
        Set-Content -LiteralPath $BodyPath -Value $scriptText -Encoding utf8
        Set-Content -LiteralPath $RunnerPath -Value $Runner -Encoding utf8
    }

    # Persistent task: created via schtasks.exe (native RPC, works even when
    # the guest CIM channel is blocked by the egress wall).  The task action
    # always runs C:\s4\tools\elev-runner.ps1, which executes the current
    # elev-body.ps1.
    #
    # S4 finding: the run-user spawn path (CreateProcessAsUserW with
    # extended startup info for AppContainer/Job) requires
    # SeAssignPrimaryTokenPrivilege, which only SYSTEM task tokens carry
    # (HL admin task tokens lack it; CreateProcessWithTokenW rejects
    # extended startup info with ERROR_INVALID_PARAMETER).  The job account
    # is therefore forced to SYSTEM.
    $taskReady = Invoke-Command -Session $sess -ArgumentList $taskName -ScriptBlock {
        param($TaskName)
        $tr = '"powershell.exe -NoProfile -ExecutionPolicy Bypass -File C:\s4\tools\elev-runner.ps1"'
        # Replacing a task that is still marked Running can fail with access
        # denied; delete first and retry creation a few times.
        $created = $false
        for ($i = 0; $i -lt 5 -and -not $created; $i++) {
            & schtasks.exe /delete /tn $TaskName /f 2>&1 | Out-Null
            & schtasks.exe /create /f /tn $TaskName /tr $tr /sc once /st 23:59 `
                /ru SYSTEM /rl highest 2>&1 | Out-Null
            if ($LASTEXITCODE -eq 0) {
                $created = $true
                break
            }
            Start-Sleep -Seconds 4
        }
        if (-not $created) { throw "schtasks create failed" }
        return $true
    }
    if (-not $taskReady) { throw "s4elevjob task not ready" }
    $started = Invoke-Command -Session $sess -ArgumentList $taskName -ScriptBlock {
        param($TaskName)
        $out = & schtasks.exe /run /tn $TaskName 2>&1
        return @{ code = $LASTEXITCODE; out = ($out -join ' ') }
    }
    if ($started.code -ne 0) {
        throw "schtasks run failed: $($started.out)"
    }

    $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
    $done = $false
    while ((Get-Date) -lt $deadline) {
        Start-Sleep -Seconds 2
        $hasExit = Invoke-Command -Session $sess -ArgumentList $logPath -ScriptBlock {
            param($LogPath)
            if (Test-Path -LiteralPath $LogPath) {
                $last = Get-Content -LiteralPath $LogPath -Tail 1
                if ($last -match '^EXIT=') { return $true }
            }
            return $false
        }
        if ($hasExit) {
            $done = $true
            break
        }
    }

    if ($done) {
        $logContent = Invoke-Command -Session $sess -ArgumentList $logPath -ScriptBlock {
            param($LogPath) Get-Content -LiteralPath $LogPath
        }
        foreach ($l in $logContent) { $lines.Add($l) }
        # 2026-09-07: propagate the guest body exit code.  A body exit != 0
        # (e.g. the agent runner exiting AGENT_LIVE_EXIT=1) must fail this
        # runner instead of surfacing as STATUS=OK (this false green masked
        # the hollow $workdir agent run in W2 chunk1).
        $exitLine = @($logContent) | Where-Object { $_ -match '^EXIT=\d+\s*$' } | Select-Object -Last 1
        if ($exitLine -and $exitLine -match '^EXIT=(\d+)\s*$' -and ([int]$Matches[1]) -ne 0) {
            $failed = $true
        }
    } else {
        $lines.Add("JOB_TIMEOUT after $TimeoutSeconds seconds")
        $failed = $true
    }
    try {
        Invoke-Command -Session $sess -ArgumentList $taskName -ScriptBlock {
            param($TaskName)
            & schtasks.exe /delete /tn $TaskName /f 2>&1 | Out-Null
        }
    }
    catch {
        $lines.Add("TASK_CLEANUP_ERR: $($_.Exception.Message)")
    }

}
catch {
    $lines.Add("ERROR: $($_.Exception.Message)")
    $lines.Add("STACK: $($_.ScriptStackTrace)")
    $failed = $true
}
finally {
    if ($sess) {
        try {
            Remove-PSSession -Session $sess -ErrorAction SilentlyContinue
        } catch {
            $lines.Add("SESSION_CLOSE_ERR: $($_.Exception.Message)")
        }
    }
}
$lines | Set-Content -LiteralPath $OutFile -Encoding utf8
if ($failed) {
    exit 1
}
exit 0
