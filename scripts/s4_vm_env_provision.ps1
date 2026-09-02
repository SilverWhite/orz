<#
    S4 F4: wall guest task-environment provisioning (host-side elevated,
    bridge op vm-env-provision).

    Installs into win-s4 (live state, no checkpoint restore) the toolchains /
    data the TB2.1 wrong-problem-set needs so tasks are attemptable inside
    the high-nist wall:

      - Python 3.12 (machine-wide): numpy, pandas, pyarrow, fasttext-wheel
        (offline wheels staged from host into C:\s4\env-provision\wheels)
      - R 4.6.1 (silent install, C:\Program Files\R\R-4.6.1)
      - node (staged under C:\s4\env-provision, installed to Program Files
        by the SYSTEM job)
      - mingw64 gcc/g++/make (staged under C:\s4\env-provision, installed to
        Program Files by the SYSTEM job)
      - train-fasttext task inputs: yelp_review_full train+test parquet under
        agent-tasks-tb2.1\train-fasttext\inputs\data\ (+ inputs-manifest,
        task.json input_mode -> inputs-present)

    Idempotent: skips copies whose destination hash already matches, and the
    guest job only installs missing pieces.  Evidence: <OutFile>.  ASCII only.
#>
[CmdletBinding()]
param(
    [string]$OutFile = 'D:\CLI\_windows_high_nist\vm-env-provision-result.txt'
)

$ErrorActionPreference = 'Stop'
$lines = New-Object System.Collections.Generic.List[string]
$failed = $false
$utf8NoBom = New-Object System.Text.UTF8Encoding($false)

function Get-FileSha256([string]$Path) {
    return (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}

try {
    $vmName = 'win-s4'
    $sec = ConvertTo-SecureString '123456' -AsPlainText -Force
    $cred = New-Object System.Management.Automation.PSCredential('HL', $sec)
    $sess = $null
    $deadline = (Get-Date).AddSeconds(240)
    while (-not $sess -and (Get-Date) -lt $deadline) {
        try {
            $sess = New-PSSession -VMName $vmName -Credential $cred -ErrorAction Stop
        }
        catch {
            Start-Sleep -Seconds 10
        }
    }
    if (-not $sess) {
        throw 'win-s4 not reachable within 240s'
    }

    $hostRoot = 'D:\CLI\_windows_high_nist'
    $envRoot = Join-Path $hostRoot 'env-provision'
    $wheelSrc = Join-Path $envRoot 'wheels'
    $taskSetDir = Join-Path $hostRoot 'agent-tasks-tb2.1'

    # ---- 0) host-side task tree update (train-fasttext data + notes) ----
    $tfId = 'train-fasttext'
    $tfDir = Join-Path $taskSetDir $tfId
    $tfInputs = Join-Path $tfDir 'inputs'
    $tfData = Join-Path $tfInputs 'data'
    New-Item -ItemType Directory -Path $tfData -Force | Out-Null
    foreach ($f in @('train-00000-of-00001.parquet', 'test-00000-of-00001.parquet')) {
        $src = Join-Path (Join-Path $envRoot 'data') $f
        if (-not (Test-Path -LiteralPath $src)) {
            throw "missing staged data: $src"
        }
        Copy-Item -LiteralPath $src -Destination (Join-Path $tfData $f) -Force
    }
    $tfFiles = New-Object System.Collections.ArrayList
    foreach ($f in (Get-ChildItem -LiteralPath $tfData -Recurse -File)) {
        $rel = $f.FullName.Substring($tfInputs.Length + 1)
        [void]$tfFiles.Add([ordered]@{
            path = $rel
            size = $f.Length
            sha256 = Get-FileSha256 $f.FullName
        })
    }
    $tfManifest = [ordered]@{
        schema_version = '0.1.0'
        task_id = $tfId
        input_mode = 'inputs-present'
        input_note = 'yelp_review_full train+test parquet provisioned under data/ (F4 env provisioning; official /app/data mirror)'
        generated_at = (Get-Date -Format 'yyyy-MM-dd HH:mm:ss')
        files = $tfFiles
    }
    [System.IO.File]::WriteAllText(
        (Join-Path $tfDir 'inputs-manifest.json'),
        ($tfManifest | ConvertTo-Json -Depth 8),
        $utf8NoBom
    )
    $tj = Get-Content -LiteralPath (Join-Path $tfDir 'task.json') -Raw -Encoding UTF8 | ConvertFrom-Json
    $tj | Add-Member -NotePropertyName 'input_mode' -NotePropertyValue 'inputs-present' -Force
    $tj | Add-Member -NotePropertyName 'inputs_manifest' -NotePropertyValue 'inputs-manifest.json' -Force
    $tj | Add-Member -NotePropertyName 'input_note' -NotePropertyValue 'yelp train+test parquet under inputs\data (F4); fasttext python module provisioned machine-wide' -Force
    [System.IO.File]::WriteAllText((Join-Path $tfDir 'task.json'), ($tj | ConvertTo-Json -Depth 10), $utf8NoBom)

    $arId = 'adaptive-rejection-sampler'
    $arTjPath = Join-Path (Join-Path $taskSetDir $arId) 'task.json'
    $arTj = Get-Content -LiteralPath $arTjPath -Raw -Encoding UTF8 | ConvertFrom-Json
    $arTj | Add-Member -NotePropertyName 'input_note' -NotePropertyValue 'no agent-facing input files; R 4.6.1 provisioned machine-wide (F4)' -Force
    [System.IO.File]::WriteAllText($arTjPath, ($arTj | ConvertTo-Json -Depth 10), $utf8NoBom)

    $manifestPath = Join-Path $taskSetDir 'manifest.json'
    $m = Get-Content -LiteralPath $manifestPath -Raw -Encoding UTF8 | ConvertFrom-Json
    foreach ($entry in $m.tasks) {
        if ([string]$entry.id -eq $tfId) {
            $entry | Add-Member -NotePropertyName 'input_mode' -NotePropertyValue 'inputs-present' -Force
        }
    }
    [System.IO.File]::WriteAllText($manifestPath, ($m | ConvertTo-Json -Depth 10), $utf8NoBom)
    $lines.Add('HOST_TASK_TREE_UPDATED train-fasttext inputs-present')

    # ---- 1) sync wheels + R installer + task tree into guest ------------
    Invoke-Command -Session $sess -ArgumentList 'C:\s4\env-provision' -ScriptBlock {
        param($root)
        New-Item -ItemType Directory -Path $root -Force | Out-Null
    }
    $wheelNeedSync = Invoke-Command -Session $sess -ScriptBlock {
        $dir = 'C:\s4\env-provision\wheels'
        if (-not (Test-Path -LiteralPath $dir)) { return $true }
        $files = @(Get-ChildItem -LiteralPath $dir -File -Filter '*.whl')
        if ($files.Count -ne 9) { return $true }
        return -not (Test-Path -LiteralPath (Join-Path $dir 'fasttext_wheel-0.9.2-cp312-cp312-win_amd64.whl'))
    }
    if ($wheelNeedSync) {
        Invoke-Command -Session $sess -ScriptBlock {
            Remove-Item -LiteralPath 'C:\s4\env-provision\wheels' -Recurse -Force -ErrorAction SilentlyContinue
        }
        Copy-Item -LiteralPath $wheelSrc -Destination 'C:\s4\env-provision' -Recurse -Force -ToSession $sess
        $lines.Add('SYNC_WHEELS copied=1')
    }
    else {
        $lines.Add('SYNC_WHEELS skip=1')
    }
    $rSrc = Join-Path (Join-Path $envRoot 'r') 'R-4.6.1-win.exe'
    $rNeedSync = Invoke-Command -Session $sess -ArgumentList (Get-FileSha256 $rSrc) -ScriptBlock {
        param($localHash)
        $p = 'C:\s4\env-provision\r\R-4.6.1-win.exe'
        if (-not (Test-Path -LiteralPath $p)) { return $true }
        return ((Get-FileHash -LiteralPath $p -Algorithm SHA256).Hash -ne $localHash)
    }
    if ($rNeedSync) {
        Invoke-Command -Session $sess -ScriptBlock {
            Remove-Item -LiteralPath 'C:\s4\env-provision\r' -Recurse -Force -ErrorAction SilentlyContinue
            New-Item -ItemType Directory -Path 'C:\s4\env-provision\r' -Force | Out-Null
        }
        Copy-Item -LiteralPath (Join-Path $envRoot 'r') -Destination 'C:\s4\env-provision' -Recurse -Force -ToSession $sess
        $lines.Add('SYNC_R copied=1')
    }
    else {
        $lines.Add('SYNC_R skip=1')
    }
    $remoteWheelCount = Invoke-Command -Session $sess -ScriptBlock {
        @(Get-ChildItem -LiteralPath 'C:\s4\env-provision\wheels' -File -Filter '*.whl').Count
    }
    $lines.Add("SYNC_WHEELS remote_count=$remoteWheelCount local_count=$(@(Get-ChildItem -LiteralPath $wheelSrc -File -Filter '*.whl').Count)")
    Copy-Item -LiteralPath $taskSetDir -Destination 'C:\s4\_windows_high_nist' -Recurse -Force -ToSession $sess
    $lines.Add('SYNC_TASKSET tb2.1 -> C:\s4\_windows_high_nist ok')

    # node (portable, staged under C:\s4\env-provision)
    $nodeSrc = 'B:\Node.js'
    $nodeRemoteHash = Invoke-Command -Session $sess -ScriptBlock {
        if (Test-Path -LiteralPath 'C:\s4\env-provision\node\node.exe') {
            (Get-FileHash -LiteralPath 'C:\s4\env-provision\node\node.exe' -Algorithm SHA256).Hash
        } else { '<missing>' }
    }
    $nodeNeedSync = ($nodeRemoteHash -ne (Get-FileSha256 (Join-Path $nodeSrc 'node.exe')))
    if ($nodeNeedSync) {
        Invoke-Command -Session $sess -ScriptBlock {
            Remove-Item -LiteralPath 'C:\s4\env-provision\node' -Recurse -Force -ErrorAction SilentlyContinue
            Remove-Item -LiteralPath 'C:\s4\env-provision\Node.js' -Recurse -Force -ErrorAction SilentlyContinue
        }
        Copy-Item -LiteralPath $nodeSrc -Destination 'C:\s4\env-provision' -Recurse -Force -ToSession $sess
        Invoke-Command -Session $sess -ScriptBlock {
            if ((Test-Path -LiteralPath 'C:\s4\env-provision\node') -eq $false) {
                Move-Item -LiteralPath 'C:\s4\env-provision\Node.js' -Destination 'C:\s4\env-provision\node' -Force
            }
        }
        $nodeRemoteHash = Invoke-Command -Session $sess -ScriptBlock {
            (Get-FileHash -LiteralPath 'C:\s4\env-provision\node\node.exe' -Algorithm SHA256).Hash
        }
        $ok = ($nodeRemoteHash -eq (Get-FileSha256 (Join-Path $nodeSrc 'node.exe')))
        $lines.Add("SYNC_NODE copied=1 ok=$ok")
        if (-not $ok) {
            throw 'node staging failed (hash mismatch)'
        }
    }
    else {
        $lines.Add('SYNC_NODE skip=1')
    }

    # mingw64 (host WinLibs)
    $gccSrc = 'B:\Toolchains\WinLibs\mingw64\bin\gcc.exe'
    $gccRemoteHash = Invoke-Command -Session $sess -ScriptBlock {
        if (Test-Path -LiteralPath 'C:\s4\env-provision\mingw64\bin\gcc.exe') {
            (Get-FileHash -LiteralPath 'C:\s4\env-provision\mingw64\bin\gcc.exe' -Algorithm SHA256).Hash
        } else { '<missing>' }
    }
    $gccNeedSync = ($gccRemoteHash -ne (Get-FileSha256 $gccSrc))
    if ($gccNeedSync) {
        Invoke-Command -Session $sess -ScriptBlock {
            Remove-Item -LiteralPath 'C:\s4\env-provision\mingw64' -Recurse -Force -ErrorAction SilentlyContinue
        }
        Copy-Item -LiteralPath 'B:\Toolchains\WinLibs\mingw64' -Destination 'C:\s4\env-provision' -Recurse -Force -ToSession $sess
        $gccRemoteHash = Invoke-Command -Session $sess -ScriptBlock {
            (Get-FileHash -LiteralPath 'C:\s4\env-provision\mingw64\bin\gcc.exe' -Algorithm SHA256).Hash
        }
        $ok = ($gccRemoteHash -eq (Get-FileSha256 $gccSrc))
        $lines.Add("SYNC_MINGW copied=1 ok=$ok")
        if (-not $ok) {
            throw 'mingw staging failed (hash mismatch)'
        }
    }
    else {
        $lines.Add('SYNC_MINGW skip=1')
    }
}
finally {
    if ($sess) {
        Remove-PSSession -Session $sess -ErrorAction SilentlyContinue
    }
}

# ---- 2) guest elevated job: offline installs + PATH + verify ------------
try {
    $ts = Get-Date -Format 'yyyyMMdd_HHmmss'
    $bodyFile = Join-Path 'D:\CLI\_windows_high_nist' ("job-env-provision-" + $ts + '-body.ps1')
    $jobOut = Join-Path 'D:\CLI\_windows_high_nist' ("job-env-provision-" + $ts + '.txt')
    $body = @'
$ErrorActionPreference = 'Continue'
$OutputEncoding = [System.Text.Encoding]::UTF8
try { [Console]::OutputEncoding = [System.Text.Encoding]::UTF8 } catch { }
$py = 'C:\Program Files\Python312\python.exe'
$wh = 'C:\s4\env-provision\wheels'

Write-Output '=== PYTHON OFFLINE INSTALL ==='
$pkgs = @('numpy', 'pandas', 'pyarrow', 'fasttext-wheel')
& $py -m pip install --no-index --find-links $wh --no-cache-dir $pkgs 2>&1 | ForEach-Object { Write-Output ("PIP: " + $_) }
Write-Output ("PIP_EXIT=" + $LASTEXITCODE)
& $py -c "import numpy, pandas, pyarrow, fasttext; print('IMPORTS_OK numpy', numpy.__version__, 'pandas', pandas.__version__, 'pyarrow', pyarrow.__version__, 'fasttext', getattr(fasttext,'__version__','n/a'))" 2>&1 |
    ForEach-Object { Write-Output ("PYCHECK: " + $_) }

Write-Output '=== R SILENT INSTALL ==='
$rInstaller = 'C:\s4\env-provision\r\R-4.6.1-win.exe'
$rRoot = 'C:\Program Files\R\R-4.6.1'
$rscript = Join-Path $rRoot 'bin\Rscript.exe'
if (Test-Path -LiteralPath $rscript) {
    Write-Output 'R_PRESENT=1'
}
else {
    if (-not (Test-Path -LiteralPath $rInstaller)) {
        Write-Output 'R_INSTALLER_MISSING=1'
    }
    else {
        $p = Start-Process -FilePath $rInstaller -ArgumentList @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', '/SP-', ('/DIR="' + $rRoot + '"')) -PassThru -Wait -WindowStyle Hidden
        Write-Output ("R_INSTALL_EXIT=" + $p.ExitCode)
    }
}
Write-Output ("RSCRIPT_PRESENT=" + (Test-Path -LiteralPath $rscript))

Write-Output '=== NODE / MINGW -> PROGRAM FILES ==='
foreach ($pair in @(
    @{ Src = 'C:\s4\env-provision\node'; Dst = 'C:\Program Files\nodejs' },
    @{ Src = 'C:\s4\env-provision\mingw64'; Dst = 'C:\Program Files\mingw64' }
)) {
    New-Item -ItemType Directory -Path $pair.Dst -Force | Out-Null
    Copy-Item -Path (Join-Path $pair.Src '*') -Destination $pair.Dst -Recurse -Force
    $probe = Join-Path $pair.Dst $(if ($pair.Dst -like '*nodejs') { 'node.exe' } else { 'bin\gcc.exe' })
    Write-Output ("INSTALL " + $pair.Dst + " probe_present=" + (Test-Path -LiteralPath $probe))
}

Write-Output '=== PATH MACHINE UPDATE ==='
$cur = [Environment]::GetEnvironmentVariable('Path', 'Machine')
$adds = @('C:\Program Files\nodejs', 'C:\Program Files\mingw64\bin', (Join-Path $rRoot 'bin\x64'), (Join-Path $rRoot 'bin'))
foreach ($a in $adds) {
    if ($cur -split ';' -notcontains $a) {
        $cur = $cur.TrimEnd(';') + ';' + $a
    }
}
[Environment]::SetEnvironmentVariable('Path', $cur, 'Machine')
$env:Path = [Environment]::GetEnvironmentVariable('Path', 'Machine')
Write-Output ("PATH_HAS_NODE=" + (($env:Path -split ';') -contains 'C:\Program Files\nodejs'))
Write-Output ("PATH_HAS_MINGW=" + (($env:Path -split ';') -contains 'C:\Program Files\mingw64\bin'))
Write-Output ("PATH_HAS_R=" + (($env:Path -split ';') -contains (Join-Path $rRoot 'bin\x64')))

Write-Output '=== TOOL VERIFY (SYSTEM) ==='
& 'C:\Program Files\nodejs\node.exe' --version 2>&1 | ForEach-Object { Write-Output ("NODE: " + $_) }
& 'C:\Program Files\mingw64\bin\gcc.exe' --version 2>&1 | Select-Object -First 1 | ForEach-Object { Write-Output ("GCC: " + $_) }
& 'C:\Program Files\mingw64\bin\g++.exe' --version 2>&1 | Select-Object -First 1 | ForEach-Object { Write-Output ("GXX: " + $_) }
if (Test-Path -LiteralPath $rscript) {
    & $rscript --version 2>&1 | Select-Object -First 1 | ForEach-Object { Write-Output ("RSCRIPT: " + $_) }
}

Write-Output '=== WALL SPAWN VERIFY (sandbox arms) ==='
$cli = 'C:\s4\scripts\run_windows_native_sandbox_command.py'
$wallPy = 'C:\s4\env-provision\wall_check.py'
$wallCode = @(
    'import os',
    "print('PY_IMPORT_OK')",
    'import numpy, pandas, pyarrow, fasttext',
    "print('MODULES', numpy.__version__, pandas.__version__, pyarrow.__version__, getattr(fasttext, '__version__', 'n/a'))"
)
$wallCode | Set-Content -LiteralPath $wallPy -Encoding utf8
foreach ($case in @(
    @{ Arm = 'non-admin'; Label = 'nonadmin' },
    @{ Arm = 'high-nist'; Label = 'hn-noac' }
)) {
    $ws = 'C:\workspace\env-provision-' + $case.Label
    Remove-Item -LiteralPath $ws -Recurse -Force -ErrorAction SilentlyContinue
    New-Item -ItemType Directory -Path $ws -Force | Out-Null
    $marker = Join-Path $ws '.assurance-p2-disposable.json'
    '{"schema_version":"0.1.0-draft","purpose":"windows-native-sandbox-probe","allow_container_write_probe":true}' |
        Set-Content -LiteralPath $marker -Encoding ascii
    $cmdPath = Join-Path $ws 'check.cmd'
    $outTxt = Join-Path $ws 'out.txt'
    $cmdText = '@echo off' + "`r`n" +
        'set FAIL=0' + "`r`n" +
        'if exist "C:\Program Files\nodejs\node.exe" ("C:\Program Files\nodejs\node.exe" --version > "' + $outTxt + '" 2>&1) else (echo NODE_MISSING >> "' + $outTxt + '" & set FAIL=1)' + "`r`n" +
        'if exist "C:\Program Files\mingw64\bin\gcc.exe" ("C:\Program Files\mingw64\bin\gcc.exe" --version >> "' + $outTxt + '" 2>&1) else (echo GCC_MISSING >> "' + $outTxt + '" & set FAIL=1)' + "`r`n" +
        'if exist "' + $rscript + '" ("' + $rscript + '" --version >> "' + $outTxt + '" 2>&1) else (echo RSCRIPT_MISSING >> "' + $outTxt + '" & set FAIL=1)' + "`r`n" +
        '"C:\Program Files\Python312\python.exe" "' + $wallPy + '" >> "' + $outTxt + '" 2>&1' + "`r`n" +
        'if %ERRORLEVEL% NEQ 0 set FAIL=1' + "`r`n" +
        'echo WALL_CHECK_DONE >> "' + $outTxt + '"' + "`r`n" +
        'exit /b %FAIL%'
    Set-Content -LiteralPath $cmdPath -Value $cmdText -Encoding ascii
    $argsList = @(
        '--workspace', $ws,
        '--arm', $case.Arm,
        '--timeout', '120',
        '--output', (Join-Path $ws 'obs.json')
    )
    if ($case.Arm -eq 'high-nist') {
        $argsList += '--no-appcontainer'
        $argsList += @('--allowlist-ip', '221.204.163.76')
    }
    $argsList += @('--command', 'cmd.exe', '/d', '/c', $cmdPath)
    & 'C:\Program Files\Python312\python.exe' $cli @argsList 2>&1 |
        ForEach-Object { Write-Output ("WALL_" + $case.Label + ": " + $_) }
    Write-Output ("WALL_" + $case.Label + "_EXIT=" + $LASTEXITCODE)
    if (Test-Path -LiteralPath $outTxt) {
        Write-Output ("WALL_" + $case.Label + "_OUT:")
        Get-Content -LiteralPath $outTxt -Encoding UTF8 | ForEach-Object { Write-Output ("  " + $_) }
    }
    $obsPath = Join-Path $ws 'obs.json'
    if (Test-Path -LiteralPath $obsPath) {
        $o = Get-Content -LiteralPath $obsPath -Raw -Encoding UTF8 | ConvertFrom-Json
        Write-Output ("WALL_" + $case.Label + "_OUTCOME=" + $o.outcome)
    }
    if (Test-Path -LiteralPath $outTxt) {
        Get-Content -LiteralPath $outTxt -Encoding UTF8 | ForEach-Object { Write-Output ("WALL_" + $case.Label + "_VERIFY " + $_) }
    }
}

Write-Output '=== TASK DATA (guest tree) ==='
$dataDir = 'C:\s4\_windows_high_nist\agent-tasks-tb2.1\train-fasttext\inputs\data'
if (Test-Path -LiteralPath $dataDir) {
    Get-ChildItem -LiteralPath $dataDir -File | Sort-Object Name | ForEach-Object {
        $h = (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash
        Write-Output ("DATA_FILE name=" + $_.Name + " size=" + $_.Length + " sha=" + $h)
    }
}
else {
    Write-Output 'DATA_DIR_MISSING=1'
}

Write-Output 'ENV_PROVISION_JOB_DONE'
exit 0
'@
    Set-Content -LiteralPath $bodyFile -Value $body -Encoding utf8
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File 'D:\CLI\scripts\s4_vm_run_elev.ps1' `
        -JobBodyFile $bodyFile -OutFile $jobOut -TimeoutSeconds 1800
    $jobExit = $LASTEXITCODE
    $lines.Add("JOB_EXIT=$jobExit")
    if (Test-Path -LiteralPath $jobOut) {
        foreach ($l in (Get-Content -LiteralPath $jobOut -Encoding UTF8)) {
            $lines.Add($l)
        }
    }
    if ($jobExit -ne 0) {
        $failed = $true
    }
}
catch {
    $lines.Add("ERROR: $($_.Exception.Message)")
    $failed = $true
}

$lines.Add("ENV_PROVISION_HOST_OK=$(-not $failed)")
$lines | Set-Content -LiteralPath $OutFile -Encoding utf8
Write-Output "resultFile=$OutFile"
if ($failed) {
    exit 1
}
exit 0
