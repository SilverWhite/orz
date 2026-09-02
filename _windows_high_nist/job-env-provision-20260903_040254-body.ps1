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
