<#
    S4 admin bridge worker (host-side elevated).

    Polls <BridgeRoot>\requests for request-<id>.json files:
      {
        "id":    "<same id>",
        "token": "<token from token.txt>",
      "op":    "ping" | "vm-state" | "vm-start" | "vm-connect" | "vm-copy" | "vm-revert" | "vm-applocker-reset" | "vm-install-clash" | "vm-checkpoint" | "vm-hive-diag" | "vm-read-wrapup" | "vm-cred-lm" | "vm-cred-inject" | "vm-sync-orz" | "vm-agent" | "vm-diag-orz" | "vm-diag-signer" | "vm-acaf-reprovision" | "vm-env-probe" | "vm-wf12-probe" | "vm-env-provision" | "check-setup" | "stage" | "quit",
      "checkpoint": "<snapshot name, op=vm-checkpoint>",
      "src":   "<host path, op=vm-copy>",
      "dst":   "<guest path, op=vm-copy>",
      "stage": "all"                                   (only when op=stage),
      "checkpoint": "<optional driver checkpoint name, op=stage>"
      }
    Responses are written to <BridgeRoot>\results\result-<id>.txt and the
    request file is renamed to done-<id>.json.

    Allowlist only: no arbitrary command text is accepted.  Ops map 1:1 to
    the fixed S4 host-side scripts.  Start this worker with
    s4_admin_bridge_start.ps1 from an elevated PowerShell.
    ASCII only.
#>
[CmdletBinding()]
param(
    [string]$BridgeRoot = 'D:\CLI\_windows_high_nist\bridge',
    [int]$PollSeconds = 2
)

$ErrorActionPreference = 'Stop'
$psExe = Join-Path $env:SystemRoot 'System32\WindowsPowerShell\v1.0\powershell.exe'
$s4Root = 'D:\CLI\_windows_high_nist'
$formalDir = Join-Path $s4Root 'formal-2026-09-02'
$reqDir = Join-Path $BridgeRoot 'requests'
$resDir = Join-Path $BridgeRoot 'results'
$logFile = Join-Path $BridgeRoot 'worker.log'
$heartFile = Join-Path $BridgeRoot 'heartbeat.txt'
$tokenFile = Join-Path $BridgeRoot 'token.txt'
$pidFile = Join-Path $BridgeRoot 'worker.pid'

foreach ($d in @($BridgeRoot, $reqDir, $resDir, $formalDir)) {
    if (-not (Test-Path -LiteralPath $d)) {
        New-Item -ItemType Directory -Path $d -Force | Out-Null
    }
}
if (-not (Test-Path -LiteralPath $tokenFile)) {
    throw "token file missing: $tokenFile (run s4_admin_bridge_start.ps1 first)"
}

function Write-Log([string]$msg) {
    Add-Content -LiteralPath $logFile -Value ("{0} {1}" -f (Get-Date -Format 'yyyy-MM-dd HH:mm:ss'), $msg) -Encoding utf8
}

function Get-ExpectedToken {
    return ((Get-Content -LiteralPath $tokenFile -Raw).Trim())
}

function Test-Token([string]$candidate) {
    $expected = Get-ExpectedToken
    if ($candidate.Length -ne $expected.Length) { return $false }
    $d = 0
    for ($i = 0; $i -lt $expected.Length; $i++) {
        $d = $d -bor ([int][char]$candidate[$i] -bxor [int][char]$expected[$i])
    }
    return ($d -eq 0)
}

function Set-Heartbeat([string]$state, [string]$id = '') {
    @("STATE=$state", "ID=$id", "TS=$((Get-Date).ToString('o'))") |
        Set-Content -LiteralPath $heartFile -Encoding utf8
}

function Write-Result([string]$id, [string[]]$content) {
    $tmp = Join-Path $resDir ("result-$id.txt.tmp")
    $final = Join-Path $resDir ("result-$id.txt")
    Set-Content -LiteralPath $tmp -Value $content -Encoding utf8
    Move-Item -LiteralPath $tmp -Destination $final -Force
}

function Invoke-FixedScript {
    param(
        [string]$ScriptPath,
        [string[]]$ArgsList = @(),
        [int]$TimeoutMs
    )
    $tmpOut = Join-Path $env:TEMP ("s4br-out-" + [guid]::NewGuid().ToString('N') + '.txt')
    $tmpErr = Join-Path $env:TEMP ("s4br-err-" + [guid]::NewGuid().ToString('N') + '.txt')
    $wrapper = Join-Path $env:TEMP ("s4br-wrap-" + [guid]::NewGuid().ToString('N') + '.ps1')
    $sidecar = Join-Path $env:TEMP ("s4br-code-" + [guid]::NewGuid().ToString('N') + '.txt')
    # Note: under 'powershell -File', Start-Process objects report a null
    # ExitCode, so the wrapper runs the fixed script as a native child and
    # persists $LASTEXITCODE to a sidecar file for the parent to read.
    $quotedArgs = @($ArgsList | ForEach-Object { "'" + ($_ -replace "'", "''") + "'" })
    $targetArg = "'" + ($ScriptPath -replace "'", "''") + "'"
    $sidecarArg = "'" + ($sidecar -replace "'", "''") + "'"
    $cmdLine = '& $psExe -NoProfile -ExecutionPolicy Bypass -File ' + $targetArg
    if ($quotedArgs.Count -gt 0) {
        $cmdLine += ' ' + ($quotedArgs -join ' ')
    }
    $wrapperBody = @(
        '$psExe = Join-Path $env:SystemRoot ''System32\WindowsPowerShell\v1.0\powershell.exe''',
        $cmdLine,
        '$c = $LASTEXITCODE',
        ('[System.IO.File]::WriteAllText(' + $sidecarArg + ', ''EXITCODE='' + $c, [System.Text.Encoding]::ASCII)'),
        'exit $c'
    ) -join "`r`n"
    [System.IO.File]::WriteAllText($wrapper, $wrapperBody, (New-Object System.Text.ASCIIEncoding))
    $p = Start-Process -FilePath $psExe `
        -ArgumentList @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', $wrapper) `
        -WindowStyle Hidden -PassThru `
        -RedirectStandardOutput $tmpOut -RedirectStandardError $tmpErr
    $timedOut = -not $p.WaitForExit($TimeoutMs)
    $code = $null
    if ($timedOut) {
        try { & taskkill.exe /PID $p.Id /T /F 2>&1 | Out-Null } catch { }
        $code = -1
    }
    $outLines = @()
    if (Test-Path -LiteralPath $tmpOut) {
        $outLines += Get-Content -LiteralPath $tmpOut -Encoding UTF8
    }
    if (Test-Path -LiteralPath $tmpErr) {
        $errLines = @(Get-Content -LiteralPath $tmpErr -Encoding UTF8)
        if ($errLines.Count -gt 0) {
            $outLines += '--- STDERR ---'
            $outLines += $errLines
        }
    }
    if ($code -ne -1 -and (Test-Path -LiteralPath $sidecar)) {
        $sideText = Get-Content -LiteralPath $sidecar -Raw
        if ($sideText -match 'EXITCODE=(-?\d+)') {
            $code = [int]$Matches[1]
        }
    }
    if ($null -eq $code) {
        $code = -2
    }
    Remove-Item -LiteralPath $wrapper -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $sidecar -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $tmpOut -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $tmpErr -Force -ErrorAction SilentlyContinue
    return [pscustomobject]@{ Code = $code; TimedOut = $timedOut; Lines = $outLines }
}

function Has-Match([string[]]$lines, [string]$pattern) {
    return (@($lines | Where-Object { $_ -match $pattern }).Count -gt 0)
}

Write-Log 'bridge worker started'
Set-Heartbeat 'IDLE'

while ($true) {
    try {
        $reqFile = Get-ChildItem -LiteralPath $reqDir -Filter 'request-*.json' -File -ErrorAction Stop |
            Sort-Object LastWriteTime | Select-Object -First 1
        if (-not $reqFile) {
            Start-Sleep -Seconds $PollSeconds
            continue
        }

        $id = ''
        try {
            $req = Get-Content -LiteralPath $reqFile.FullName -Raw -Encoding UTF8 | ConvertFrom-Json
            $id = [string]$req.id
            if ([string]::IsNullOrWhiteSpace($id)) {
                $id = [guid]::NewGuid().ToString('N')
            }
            Set-Heartbeat 'BUSY' $id

            if (-not (Test-Token ([string]$req.token))) {
                Write-Log "token mismatch req=$id"
                Write-Result $id @('STATUS=FAIL', "ID=$id", 'REASON=TOKEN_MISMATCH')
            }
            else {
                $op = ([string]$req.op).ToLowerInvariant()
                switch ($op) {
                    'ping' {
                        Write-Result $id @('STATUS=OK', "ID=$id", 'OP=ping', 'PONG=1')
                    }
                    'vm-state' {
                        try {
                            $vm = Get-VM -Name 'win-s4' -ErrorAction Stop
                            Write-Result $id @('STATUS=OK', "ID=$id", 'OP=vm-state', "STATE=$($vm.State)")
                        }
                        catch {
                            Write-Result $id @('STATUS=FAIL', "ID=$id", 'OP=vm-state', "ERROR=$($_.Exception.Message)")
                        }
                    }
                    'vm-start' {
                        $r = @(Invoke-FixedScript -ScriptPath 'D:\CLI\scripts\s4_vm_start.ps1' -TimeoutMs 600000)[-1]
                        $resFile = 'D:\CLI\_windows_high_nist\vm-start-result.txt'
                        $tail = @()
                        if (Test-Path -LiteralPath $resFile) {
                            $tail = @(Get-Content -LiteralPath $resFile -Encoding UTF8)
                        }
                        $ok = (-not $r.TimedOut) -and (Has-Match $tail '^STATE_AFTER=Running$')
                        $status = $(if ($ok) { 'OK' } else { 'FAIL' })
                        $codeStr = if ($null -eq $r.Code) { '' } else { [string]$r.Code }
                        $tdStr = if ($r.TimedOut) { '1' } else { '0' }
                        $head = @("STATUS=$status", "ID=$id", 'OP=vm-start', "EXIT=$codeStr", "TIMEDOUT=$tdStr")
                        $all = @(); $all += $head; $all += @($r.Lines); $all += @($tail)
                        Write-Result $id $all
                    }
                    'vm-connect' {
                        try {
                            $vmconnect = Join-Path $env:SystemRoot 'System32\vmconnect.exe'
                            Start-Process -FilePath $vmconnect -ArgumentList @('localhost', 'win-s4')
                            Write-Result $id @('STATUS=OK', "ID=$id", 'OP=vm-connect', 'STARTED=1')
                        }
                        catch {
                            Write-Result $id @('STATUS=FAIL', "ID=$id", 'OP=vm-connect', "ERROR=$($_.Exception.Message)")
                        }
                    }
                    'vm-copy' {
                        $src = [string]$req.src
                        $dst = [string]$req.dst
                        $srcOk = $false
                        if ([string]::IsNullOrWhiteSpace($src)) {
                            Write-Result $id @('STATUS=FAIL', "ID=$id", 'OP=vm-copy', 'REASON=SRC_REQUIRED')
                        }
                        else {
                            $fullSrc = [System.IO.Path]::GetFullPath($src)
                            $allowedSrcRoot = [System.IO.Path]::GetFullPath('D:\CLI') + [System.IO.Path]::DirectorySeparatorChar
                            $desktopDir = [System.IO.Path]::GetFullPath('C:\Users\1\OneDrive\Desktop') + [System.IO.Path]::DirectorySeparatorChar
                            $srcOk = ($fullSrc.StartsWith($allowedSrcRoot, [System.StringComparison]::OrdinalIgnoreCase)) -or
                                (([System.IO.Path]::GetFileName($fullSrc) -match '^Clash.*\.exe$') -and
                                 $fullSrc.StartsWith($desktopDir, [System.StringComparison]::OrdinalIgnoreCase))
                        }
                        $dstOk = $false
                        if (-not [string]::IsNullOrWhiteSpace($dst)) {
                            $fullDst = [System.IO.Path]::GetFullPath($dst)
                            foreach ($root in @('C:\Users\HL\Desktop', 'C:\Users\HL\Downloads', 'C:\s4\tools', 'C:\workspace')) {
                                $fullRoot = [System.IO.Path]::GetFullPath($root) + [System.IO.Path]::DirectorySeparatorChar
                                if ($fullDst.StartsWith($fullRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
                                    $dstOk = $true
                                    break
                                }
                            }
                        }
                        if (-not $srcOk -or -not $dstOk -or -not (Test-Path -LiteralPath $src)) {
                            Write-Result $id @('STATUS=FAIL', "ID=$id", 'OP=vm-copy',
                                "REASON=SRC_OR_DST_NOT_ALLOWED srcOk=$srcOk dstOk=$dstOk exists=$(Test-Path -LiteralPath $src)")
                        }
                        else {
                            try {
                                $sec = ConvertTo-SecureString '123456' -AsPlainText -Force
                                $cred = New-Object System.Management.Automation.PSCredential('HL', $sec)
                                $sess = New-PSSession -VMName 'win-s4' -Credential $cred
                                try {
                                    $dstParent = Split-Path -Parent $dst
                                    Invoke-Command -Session $sess -ArgumentList $dstParent -ScriptBlock {
                                        param($DirPath)
                                        New-Item -ItemType Directory -Path $DirPath -Force | Out-Null
                                    }
                                    Copy-Item -LiteralPath $src -Destination $dst -ToSession $sess -Force
                                    $srcHash = (Get-FileHash -LiteralPath $src -Algorithm SHA256).Hash
                                    $dstHash = Invoke-Command -Session $sess -ArgumentList $dst -ScriptBlock {
                                        param($GuestPath) (Get-FileHash -LiteralPath $GuestPath -Algorithm SHA256).Hash
                                    }
                                    $hashOk = ($srcHash -eq $dstHash)
                                    if ($hashOk) {
                                        Write-Result $id @('STATUS=OK', "ID=$id", 'OP=vm-copy',
                                            "SRC=$src", "DST=$dst", "SHA256=$srcHash")
                                    }
                                    else {
                                        Write-Result $id @('STATUS=FAIL', "ID=$id", 'OP=vm-copy',
                                            'REASON=HASH_MISMATCH', "SRC_SHA=$srcHash", "DST_SHA=$dstHash")
                                    }
                                }
                                finally {
                                    Remove-PSSession -Session $sess -ErrorAction SilentlyContinue
                                }
                            }
                            catch {
                                Write-Result $id @('STATUS=FAIL', "ID=$id", 'OP=vm-copy', "ERROR=$($_.Exception.Message)")
                            }
                        }
                    }
                    'vm-revert' {
                        $r = @(Invoke-FixedScript -ScriptPath 'D:\CLI\scripts\s4_vm_revert_hardening.ps1' -TimeoutMs 600000)[-1]
                        $resFile = 'D:\CLI\_windows_high_nist\vm-revert-hn-result.txt'
                        $tail = @()
                        if (Test-Path -LiteralPath $resFile) {
                            $tail = @(Get-Content -LiteralPath $resFile -Encoding UTF8)
                        }
                        $revertOk = (Has-Match $r.Lines '^REVERT_OK=True$') -or
                            ((Has-Match $tail '^REVERT_HN_EXIT=0$') -and (Has-Match $tail '^EXIT=0$'))
                        $ok = (-not $r.TimedOut) -and ($r.Code -eq 0) -and $revertOk
                        $status = $(if ($ok) { 'OK' } else { 'FAIL' })
                        $codeStr = if ($null -eq $r.Code) { '' } else { [string]$r.Code }
                        $tdStr = if ($r.TimedOut) { '1' } else { '0' }
                        $head = @("STATUS=$status", "ID=$id", 'OP=vm-revert', "EXIT=$codeStr", "TIMEDOUT=$tdStr")
                        $all = @(); $all += $head; $all += @($r.Lines); $all += @($tail)
                        Write-Result $id $all
                    }
                    'vm-applocker-reset' {
                        $r = @(Invoke-FixedScript -ScriptPath 'D:\CLI\scripts\s4_vm_applocker_reset.ps1' -TimeoutMs 400000)[-1]
                        $resFile = 'D:\CLI\_windows_high_nist\vm-applocker-reset-result.txt'
                        $tail = @()
                        if (Test-Path -LiteralPath $resFile) {
                            $tail = @(Get-Content -LiteralPath $resFile -Encoding UTF8)
                        }
                        $ok = (-not $r.TimedOut) -and ($r.Code -eq 0) -and
                            (Has-Match $r.Lines '^APPLOCKER_RESET_OK=True$')
                        $status = $(if ($ok) { 'OK' } else { 'FAIL' })
                        $codeStr = if ($null -eq $r.Code) { '' } else { [string]$r.Code }
                        $tdStr = if ($r.TimedOut) { '1' } else { '0' }
                        $head = @("STATUS=$status", "ID=$id", 'OP=vm-applocker-reset', "EXIT=$codeStr", "TIMEDOUT=$tdStr")
                        $all = @(); $all += $head; $all += @($r.Lines); $all += @($tail)
                        Write-Result $id $all
                    }
                    'vm-install-clash' {
                        $r = @(Invoke-FixedScript -ScriptPath 'D:\CLI\scripts\s4_vm_install_clash.ps1' -TimeoutMs 500000)[-1]
                        $resFile = 'D:\CLI\_windows_high_nist\vm-install-clash-result.txt'
                        $tail = @()
                        if (Test-Path -LiteralPath $resFile) {
                            $tail = @(Get-Content -LiteralPath $resFile -Encoding UTF8)
                        }
                        $ok = (-not $r.TimedOut) -and ($r.Code -eq 0) -and
                            (Has-Match $r.Lines '^INSTALL_CLASH_OK=True$')
                        $status = $(if ($ok) { 'OK' } else { 'FAIL' })
                        $codeStr = if ($null -eq $r.Code) { '' } else { [string]$r.Code }
                        $tdStr = if ($r.TimedOut) { '1' } else { '0' }
                        $head = @("STATUS=$status", "ID=$id", 'OP=vm-install-clash', "EXIT=$codeStr", "TIMEDOUT=$tdStr")
                        $all = @(); $all += $head; $all += @($r.Lines); $all += @($tail)
                        Write-Result $id $all
                    }
                    'vm-checkpoint' {
                        $name = [string]$req.checkpoint
                        if ([string]::IsNullOrWhiteSpace($name) -or ($name -notmatch '^[A-Za-z0-9_.\-]+$')) {
                            Write-Result $id @('STATUS=FAIL', "ID=$id", 'OP=vm-checkpoint', 'REASON=BAD_NAME')
                        }
                        else {
                            try {
                                Checkpoint-VM -Name 'win-s4' -SnapshotName $name -Confirm:$false -ErrorAction Stop
                                $snap = Get-VMSnapshot -VMName 'win-s4' -Name $name -ErrorAction Stop
                                Write-Result $id @('STATUS=OK', "ID=$id", 'OP=vm-checkpoint',
                                    "CHECKPOINT=$name", "CREATED=$($snap.CreationTime)")
                            }
                            catch {
                                Write-Result $id @('STATUS=FAIL', "ID=$id", 'OP=vm-checkpoint',
                                    "ERROR=$($_.Exception.Message)")
                            }
                        }
                    }
                    'check-setup' {
                        $r = @(Invoke-FixedScript -ScriptPath 'D:\CLI\scripts\s4_vm_check_setup.ps1' -TimeoutMs 600000)[-1]
                        $resFile = 'D:\CLI\_windows_high_nist\formal-2026-09-02\setup-check.txt'
                        $tail = @()
                        if (Test-Path -LiteralPath $resFile) {
                            $tail = @(Get-Content -LiteralPath $resFile -Encoding UTF8)
                        }
                        $ok = (-not $r.TimedOut) -and ($r.Code -eq 0)
                        $status = $(if ($ok) { 'OK' } else { 'FAIL' })
                        $codeStr = if ($null -eq $r.Code) { '' } else { [string]$r.Code }
                        $tdStr = if ($r.TimedOut) { '1' } else { '0' }
                        $head = @("STATUS=$status", "ID=$id", 'OP=check-setup', "EXIT=$codeStr", "TIMEDOUT=$tdStr")
                        $all = @(); $all += $head; $all += @($r.Lines); $all += @($tail)
                        Write-Result $id $all
                    }
                    'vm-hive-diag' {
                        $r = @(Invoke-FixedScript -ScriptPath 'D:\CLI\scripts\s4_vm_hive_diag.ps1' -TimeoutMs 420000)[-1]
                        $resFile = 'D:\CLI\_windows_high_nist\vm-hive-diag-result.txt'
                        $tail = @()
                        if (Test-Path -LiteralPath $resFile) {
                            $tail = @(Get-Content -LiteralPath $resFile -Encoding UTF8)
                        }
                        $ok = (-not $r.TimedOut) -and ($r.Code -eq 0) -and
                            (Has-Match $tail '^HIVEDIAG_DONE$')
                        $status = $(if ($ok) { 'OK' } else { 'FAIL' })
                        $codeStr = if ($null -eq $r.Code) { '' } else { [string]$r.Code }
                        $tdStr = if ($r.TimedOut) { '1' } else { '0' }
                        $head = @("STATUS=$status", "ID=$id", 'OP=vm-hive-diag', "EXIT=$codeStr", "TIMEDOUT=$tdStr")
                        $all = @(); $all += $head; $all += @($r.Lines); $all += @($tail)
                        Write-Result $id $all
                    }
                    'vm-read-wrapup' {
                        $r = @(Invoke-FixedScript -ScriptPath 'D:\CLI\scripts\s4_vm_read_wrapup.ps1' -TimeoutMs 420000)[-1]
                        $resFile = 'D:\CLI\_windows_high_nist\vm-read-wrapup-result.txt'
                        $tail = @()
                        if (Test-Path -LiteralPath $resFile) {
                            $tail = @(Get-Content -LiteralPath $resFile -Encoding UTF8)
                        }
                        $ok = (-not $r.TimedOut) -and ($r.Code -eq 0)
                        $status = $(if ($ok) { 'OK' } else { 'FAIL' })
                        $codeStr = if ($null -eq $r.Code) { '' } else { [string]$r.Code }
                        $tdStr = if ($r.TimedOut) { '1' } else { '0' }
                        $head = @("STATUS=$status", "ID=$id", 'OP=vm-read-wrapup', "EXIT=$codeStr", "TIMEDOUT=$tdStr")
                        $all = @(); $all += $head; $all += @($r.Lines); $all += @($tail)
                        Write-Result $id $all
                    }
                    'vm-cred-lm' {
                        $r = @(Invoke-FixedScript -ScriptPath 'D:\CLI\scripts\s4_vm_cred_lm_probe.ps1' -TimeoutMs 1000000)[-1]
                        $resFile = 'D:\CLI\_windows_high_nist\vm-cred-lm-result.txt'
                        $tail = @()
                        if (Test-Path -LiteralPath $resFile) {
                            $tail = @(Get-Content -LiteralPath $resFile -Encoding UTF8)
                        }
                        $ok = (-not $r.TimedOut) -and ($r.Code -eq 0) -and
                            (Has-Match $tail '^CRED_LM_DONE$') -and
                            (Has-Match $tail '^CRED_LM_ERRORS=0$')
                        $status = $(if ($ok) { 'OK' } else { 'FAIL' })
                        $codeStr = if ($null -eq $r.Code) { '' } else { [string]$r.Code }
                        $tdStr = if ($r.TimedOut) { '1' } else { '0' }
                        $head = @("STATUS=$status", "ID=$id", 'OP=vm-cred-lm', "EXIT=$codeStr", "TIMEDOUT=$tdStr")
                        $all = @(); $all += $head; $all += @($r.Lines); $all += @($tail)
                        Write-Result $id $all
                    }
                    'vm-cred-inject' {
                        $r = @(Invoke-FixedScript -ScriptPath 'D:\CLI\scripts\s4_vm_cred_inject_probe.ps1' -TimeoutMs 1000000)[-1]
                        $resFile = 'D:\CLI\_windows_high_nist\vm-cred-inject-result.txt'
                        $tail = @()
                        if (Test-Path -LiteralPath $resFile) {
                            $tail = @(Get-Content -LiteralPath $resFile -Encoding UTF8)
                        }
                        $ok = (-not $r.TimedOut) -and ($r.Code -eq 0) -and
                            (Has-Match $tail '^CRED_INJECT_DONE$') -and
                            (Has-Match $tail '^CRED_INJECT_ERRORS=0$') -and
                            (Has-Match $tail '^INJECT_HASH_MATCH=True$')
                        $status = $(if ($ok) { 'OK' } else { 'FAIL' })
                        $codeStr = if ($null -eq $r.Code) { '' } else { [string]$r.Code }
                        $tdStr = if ($r.TimedOut) { '1' } else { '0' }
                        $head = @("STATUS=$status", "ID=$id", 'OP=vm-cred-inject', "EXIT=$codeStr", "TIMEDOUT=$tdStr")
                        $all = @(); $all += $head; $all += @($r.Lines); $all += @($tail)
                        Write-Result $id $all
                    }
                    'vm-sync-orz' {
                        $r = @(Invoke-FixedScript -ScriptPath 'D:\CLI\scripts\s4_vm_sync_orz.ps1' -TimeoutMs 600000)[-1]
                        $resFile = 'D:\CLI\_windows_high_nist\vm-sync-orz-result.txt'
                        $tail = @()
                        if (Test-Path -LiteralPath $resFile) {
                            $tail = @(Get-Content -LiteralPath $resFile -Encoding UTF8)
                        }
                        $ok = (-not $r.TimedOut) -and ($r.Code -eq 0) -and
                            (Has-Match $tail '^ORZ_SYNC_OK=True$')
                        $status = $(if ($ok) { 'OK' } else { 'FAIL' })
                        $codeStr = if ($null -eq $r.Code) { '' } else { [string]$r.Code }
                        $tdStr = if ($r.TimedOut) { '1' } else { '0' }
                        $head = @("STATUS=$status", "ID=$id", 'OP=vm-sync-orz', "EXIT=$codeStr", "TIMEDOUT=$tdStr")
                        $all = @(); $all += $head; $all += @($r.Lines); $all += @($tail)
                        Write-Result $id $all
                    }
                    'vm-agent' {
                        $arm = if ($req.arm) { [string]$req.arm } else { 'high-nist' }
                        if ($arm -notin @('control', 'high-nist')) {
                            Write-Result $id @('STATUS=FAIL', "ID=$id", 'OP=vm-agent', "REASON=bad arm: $arm")
                            break
                        }
                        $taskIds = [string]$req.taskIds
                        $firstId = @($taskIds -split ',' | ForEach-Object { $_.Trim() } | Where-Object { $_ } | Select-Object -First 1)
                        if (-not $firstId) {
                            Write-Result $id @('STATUS=FAIL', "ID=$id", 'OP=vm-agent', 'REASON=taskIds required')
                            break
                        }
                        $taskSet = if ($req.taskSet) { [string]$req.taskSet } else { 'tb2.1' }
                        if ($taskSet -notin @('tb2.1', 'friction')) {
                            Write-Result $id @('STATUS=FAIL', "ID=$id", 'OP=vm-agent', "REASON=bad taskSet: $taskSet")
                            break
                        }
                        $runTag = [string]$req.runTag
                        if (-not $runTag) {
                            $runTag = $firstId
                        }
                        $runTag = ($runTag -replace '[^A-Za-z0-9._-]', '_')
                        $dryRaw = ([string]$req.dryRun).ToLowerInvariant()
                        $dryRun = ($dryRaw -in @('true', '1'))
                        $argsList = @('-Arm', $arm, '-TaskIds', $taskIds, '-TaskSet', $taskSet, '-RunTag', $runTag)
                        $keyFile = [string]$req.keyFile
                        if ($keyFile) {
                            $argsList += @('-KeyFile', $keyFile)
                        }
                        if ($dryRun) {
                            $argsList += '-DryRun'
                        }
                        $timeoutMs = if ($dryRun) { 1200000 } else { 14400000 }
                        $r = @(Invoke-FixedScript -ScriptPath 'D:\CLI\scripts\s4_vm_agent_run.ps1' `
                            -ArgsList $argsList -TimeoutMs $timeoutMs)[-1]
                        $resFile = "D:\CLI\_windows_high_nist\formal-2026-09-02\vm-agent-run-$runTag.txt"
                        $tail = @()
                        if (Test-Path -LiteralPath $resFile) {
                            $tail = @(Get-Content -LiteralPath $resFile -Encoding UTF8 | Select-Object -Last 120)
                        }
                        $ok = (-not $r.TimedOut) -and ($r.Code -eq 0) -and
                            (Has-Match $tail '^AGENT_RUN_OK=True$')
                        $status = $(if ($ok) { 'OK' } else { 'FAIL' })
                        $codeStr = if ($null -eq $r.Code) { '' } else { [string]$r.Code }
                        $tdStr = if ($r.TimedOut) { '1' } else { '0' }
                        $head = @("STATUS=$status", "ID=$id", 'OP=vm-agent', "ARM=$arm", "TASKSET=$taskSet", "RUN_TAG=$runTag", "DRY_RUN=$dryRun", "EXIT=$codeStr", "TIMEDOUT=$tdStr")
                        $all = @(); $all += $head; $all += @($r.Lines); $all += @($tail)
                        Write-Result $id $all
                        Write-Log "vm-agent arm=$arm taskSet=$taskSet runTag=$runTag dry=$dryRun done exit=$codeStr timedout=$($r.TimedOut)"
                    }
                    'vm-diag-orz' {
                        $r = @(Invoke-FixedScript -ScriptPath 'D:\CLI\scripts\s4_vm_diag_orz_sweep.ps1' -TimeoutMs 900000)[-1]
                        $ok = (-not $r.TimedOut) -and ($r.Code -eq 0)
                        $status = $(if ($ok) { 'OK' } else { 'FAIL' })
                        $codeStr = if ($null -eq $r.Code) { '' } else { [string]$r.Code }
                        $tdStr = if ($r.TimedOut) { '1' } else { '0' }
                        $head = @("STATUS=$status", "ID=$id", 'OP=vm-diag-orz', "EXIT=$codeStr", "TIMEDOUT=$tdStr")
                        $all = @(); $all += $head; $all += @($r.Lines)
                        Write-Result $id $all
                        Write-Log "vm-diag-orz done exit=$codeStr timedout=$($r.TimedOut)"
                    }
                    'vm-diag-signer' {
                        $r = @(Invoke-FixedScript -ScriptPath 'D:\CLI\scripts\s4_vm_diag_signer_f1.ps1' -TimeoutMs 900000)[-1]
                        $ok = (-not $r.TimedOut) -and ($r.Code -eq 0)
                        $status = $(if ($ok) { 'OK' } else { 'FAIL' })
                        $codeStr = if ($null -eq $r.Code) { '' } else { [string]$r.Code }
                        $tdStr = if ($r.TimedOut) { '1' } else { '0' }
                        $head = @("STATUS=$status", "ID=$id", 'OP=vm-diag-signer', "EXIT=$codeStr", "TIMEDOUT=$tdStr")
                        $all = @(); $all += $head; $all += @($r.Lines)
                        Write-Result $id $all
                        Write-Log "vm-diag-signer done exit=$codeStr timedout=$($r.TimedOut)"
                    }
                    'vm-acaf-reprovision' {
                        $r = @(Invoke-FixedScript -ScriptPath 'D:\CLI\scripts\s4_vm_acaf_reprovision.ps1' -TimeoutMs 900000)[-1]
                        $resFile = 'D:\CLI\_windows_high_nist\vm-acaf-reprovision-result.txt'
                        $tail = @()
                        if (Test-Path -LiteralPath $resFile) {
                            $tail = @(Get-Content -LiteralPath $resFile -Encoding UTF8 | Select-Object -Last 80)
                        }
                        $ok = (-not $r.TimedOut) -and ($r.Code -eq 0) -and
                            (Has-Match $tail '^ACAF_REPROVISION_HOST_OK=True$') -and
                            (Has-Match $tail '^ACAF_PROVISION_OK=True$')
                        $status = $(if ($ok) { 'OK' } else { 'FAIL' })
                        $codeStr = if ($null -eq $r.Code) { '' } else { [string]$r.Code }
                        $tdStr = if ($r.TimedOut) { '1' } else { '0' }
                        $head = @("STATUS=$status", "ID=$id", 'OP=vm-acaf-reprovision', "EXIT=$codeStr", "TIMEDOUT=$tdStr")
                        $all = @(); $all += $head; $all += @($r.Lines); $all += @($tail)
                        Write-Result $id $all
                        Write-Log "vm-acaf-reprovision done exit=$codeStr timedout=$($r.TimedOut)"
                    }
                    'vm-env-probe' {
                        $r = @(Invoke-FixedScript -ScriptPath 'D:\CLI\scripts\s4_vm_env_probe.ps1' -TimeoutMs 900000)[-1]
                        $resFile = 'D:\CLI\_windows_high_nist\vm-env-probe-result.txt'
                        $tail = @()
                        if (Test-Path -LiteralPath $resFile) {
                            $tail = @(Get-Content -LiteralPath $resFile -Encoding UTF8)
                        }
                        $ok = (-not $r.TimedOut) -and ($r.Code -eq 0) -and
                            (Has-Match $tail '^ENV_PROBE_HOST_OK=True$')
                        $status = $(if ($ok) { 'OK' } else { 'FAIL' })
                        $codeStr = if ($null -eq $r.Code) { '' } else { [string]$r.Code }
                        $tdStr = if ($r.TimedOut) { '1' } else { '0' }
                        $head = @("STATUS=$status", "ID=$id", 'OP=vm-env-probe', "EXIT=$codeStr", "TIMEDOUT=$tdStr")
                        $all = @(); $all += $head; $all += @($r.Lines); $all += @($tail)
                        Write-Result $id $all
                        Write-Log "vm-env-probe done exit=$codeStr timedout=$($r.TimedOut)"
                    }
                    'vm-wf12-probe' {
                        $r = @(Invoke-FixedScript -ScriptPath 'D:\CLI\scripts\s4_vm_wf12_probe.ps1' -TimeoutMs 900000)[-1]
                        $resFile = 'D:\CLI\_windows_high_nist\vm-wf12-probe-result.txt'
                        $tail = @()
                        if (Test-Path -LiteralPath $resFile) {
                            $tail = @(Get-Content -LiteralPath $resFile -Encoding UTF8)
                        }
                        $ok = (-not $r.TimedOut) -and ($r.Code -eq 0) -and
                            (Has-Match $tail '^WF12_PROBE_HOST_OK=True$')
                        $status = $(if ($ok) { 'OK' } else { 'FAIL' })
                        $codeStr = if ($null -eq $r.Code) { '' } else { [string]$r.Code }
                        $tdStr = if ($r.TimedOut) { '1' } else { '0' }
                        $head = @("STATUS=$status", "ID=$id", 'OP=vm-wf12-probe', "EXIT=$codeStr", "TIMEDOUT=$tdStr")
                        $all = @(); $all += $head; $all += @($r.Lines); $all += @($tail)
                        Write-Result $id $all
                        Write-Log "vm-wf12-probe done exit=$codeStr timedout=$($r.TimedOut)"
                    }
                    'vm-env-provision' {
                        $r = @(Invoke-FixedScript -ScriptPath 'D:\CLI\scripts\s4_vm_env_provision.ps1' -TimeoutMs 3600000)[-1]
                        $resFile = 'D:\CLI\_windows_high_nist\vm-env-provision-result.txt'
                        $tail = @()
                        if (Test-Path -LiteralPath $resFile) {
                            $tail = @(Get-Content -LiteralPath $resFile -Encoding UTF8)
                        }
                        $ok = (-not $r.TimedOut) -and ($r.Code -eq 0) -and
                            (Has-Match $tail '^ENV_PROVISION_HOST_OK=True$') -and
                            (Has-Match $tail 'IMPORTS_OK numpy') -and
                            (Has-Match $tail '^RSCRIPT_PRESENT=True$') -and
                            (Has-Match $tail '^PATH_HAS_NODE=True$') -and
                            (Has-Match $tail '^PATH_HAS_MINGW=True$') -and
                            (Has-Match $tail '^WALL_nonadmin_OUTCOME=compliant$') -and
                            (Has-Match $tail '^WALL_hn-noac_OUTCOME=compliant$')
                        $status = $(if ($ok) { 'OK' } else { 'FAIL' })
                        $codeStr = if ($null -eq $r.Code) { '' } else { [string]$r.Code }
                        $tdStr = if ($r.TimedOut) { '1' } else { '0' }
                        $head = @("STATUS=$status", "ID=$id", 'OP=vm-env-provision', "EXIT=$codeStr", "TIMEDOUT=$tdStr")
                        $all = @(); $all += $head; $all += @($r.Lines); $all += @($tail)
                        Write-Result $id $all
                        Write-Log "vm-env-provision done exit=$codeStr timedout=$($r.TimedOut)"
                    }
                    'stage' {
                        $stage = ([string]$req.stage).ToLowerInvariant()
                        $allowed = @('restore', 'sync', 'setup', 'control', 'nonadmin', 'highnist', 'taskcontrol', 'tasknonadmin', 'taskhighnist', 'agentcontrol', 'agenthighnist', 'netcheck', 'wrapup', 'all', 'full', 'reboot')
                        if ($allowed -notcontains $stage) {
                            Write-Result $id @('STATUS=FAIL', "ID=$id", 'OP=stage', "REASON=unknown stage: $stage")
                        }
                        else {
                            $ck = ([string]$req.checkpoint).Trim()
                            $ckAllowed = @('S4-BASE-INSTALLED', 'S4-BASE-NET-2026-09-02')
                            if ($ck -and ($ckAllowed -notcontains $ck)) {
                                Write-Result $id @('STATUS=FAIL', "ID=$id", 'OP=stage', "REASON=bad checkpoint: $ck")
                            }
                            else {
                                $stageArgs = @('-Stage', $stage)
                                if ($ck) {
                                    $stageArgs += @('-CheckpointName', $ck)
                                }
                                # Model-side agent rounds run up to ~3h per
                                # job (9 tasks x ~15min worst case); machine
                                # stages keep the 45-min cap.
                                $timeoutMs = 2700000
                                if ($stage -in @('agentcontrol', 'agenthighnist')) {
                                    $timeoutMs = 14400000
                                }
                                $r = @(Invoke-FixedScript -ScriptPath 'D:\CLI\scripts\s4_vm_arms_formal.ps1' `
                                    -ArgsList $stageArgs -TimeoutMs $timeoutMs)[-1]
                                $stageFile = Join-Path $formalDir ("stage-$stage.txt")
                                $tail = @()
                                if (Test-Path -LiteralPath $stageFile) {
                                    $tail = @(Get-Content -LiteralPath $stageFile -Encoding UTF8 | Select-Object -Last 60)
                                }
                                $ok = (-not $r.TimedOut) -and ($r.Code -eq 0)
                                $status = $(if ($ok) { 'OK' } else { 'FAIL' })
                                $codeStr = if ($null -eq $r.Code) { '' } else { [string]$r.Code }
                                $tdStr = if ($r.TimedOut) { '1' } else { '0' }
                                $head = @("STATUS=$status", "ID=$id", 'OP=stage', "STAGE=$stage", "CHECKPOINT=$ck", "EXIT=$codeStr", "TIMEDOUT=$tdStr")
                                $all = @(); $all += $head; $all += @($r.Lines); $all += @($tail)
                                Write-Result $id $all
                                Write-Log "stage $stage checkpoint=$ck done exit=$codeStr timedout=$($r.TimedOut)"
                            }
                        }
                    }
                    'restart' {
                        $workerPath = $PSCommandPath
                        if (-not (Test-Path -LiteralPath $workerPath)) {
                            $workerPath = Join-Path $PSScriptRoot 's4_admin_bridge.ps1'
                        }
                        $p = Start-Process -FilePath $psExe `
                            -ArgumentList @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', $workerPath, '-BridgeRoot', $BridgeRoot) `
                            -WindowStyle Hidden -PassThru
                        try {
                            [System.IO.File]::WriteAllText($pidFile, [string]$p.Id, (New-Object System.Text.ASCIIEncoding))
                        }
                        catch { }
                        Write-Result $id @('STATUS=OK', "ID=$id", 'OP=restart', "PID=$($p.Id)")
                        Write-Log "bridge worker restart issued pid=$($p.Id)"
                        try {
                            Move-Item -LiteralPath $reqFile.FullName -Destination (Join-Path $reqDir ("done-$id.json")) -Force
                        }
                        catch { }
                        exit 0
                    }
                    'quit' {
                        Write-Result $id @('STATUS=OK', "ID=$id", 'OP=quit', 'BYE=1')
                        Move-Item -LiteralPath $reqFile.FullName -Destination (Join-Path $reqDir ("done-$id.json")) -Force
                        Set-Heartbeat 'STOPPED'
                        Write-Log 'bridge worker quit requested'
                        exit 0
                    }
                    default {
                        Write-Result $id @('STATUS=FAIL', "ID=$id", "REASON=unknown op: $op")
                    }
                }
            }
        }
        catch {
            Write-Log "request error: $($_.Exception.Message)"
            Write-Result $id @('STATUS=FAIL', "ID=$id", 'REASON=WORKER_ERROR', "ERROR=$($_.Exception.Message)")
        }
        try {
            Move-Item -LiteralPath $reqFile.FullName -Destination (Join-Path $reqDir ("done-$id.json")) -Force
        }
        catch { }
        Set-Heartbeat 'IDLE'
    }
    catch {
        Write-Log "loop error: $($_.Exception.Message)"
        Set-Heartbeat 'ERROR'
    }
    Start-Sleep -Seconds $PollSeconds
}
