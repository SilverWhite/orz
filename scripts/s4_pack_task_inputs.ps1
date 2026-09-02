<#
    S4 F2: materialize TB2.1 task input packs for the Windows friction rig.

    For every task in agent-tasks-tb2.1 this writes task.json input_mode and
    (where inputs exist) an inputs\ folder + inputs-manifest.json (sha256).
    Official /app layout is mirrored by the runner into C:\app (see
    run_agent_arm.ps1); no instruction text is rewritten.

    Materialized now (offline sources + host toolchain):
      gcode-to-text                  text.gcode (gunzip official)
      make-doom-for-mips             doomgeneric/ (pinned clone),
                                     doomgeneric_img.c, vm.js, doom.wad
      path-tracing                   image.ppm (orig.c rendered by host gcc)
      model-extraction-relu-logits   forward.py
      dna-insert                     sequences.fasta
    No-input-diagnostic (recorded only):
      mteb-leaderboard (network retrieval; egress allowlist blocks general
                        web), train-fasttext (train data not fetched +
                        fasttext absent), adaptive-rejection-sampler (R
                        toolchain + verifier-only artifact),
      filter-js-from-html (write-code task; verifier supplies HTML).

    Needs network for make-doom (github + ibiblio); run elevated when the
    sandbox has no route.  ASCII console output only.
#>
[CmdletBinding()]
param(
    [string]$TasksRoot = 'D:\CLI\_windows_high_nist\agent-tasks-tb2.1',
    [string]$OfficialRoot = 'D:\tb-eval\terminal-bench-2-1\tasks',
    [string]$CacheDir = 'D:\CLI\_windows_high_nist\pack-cache',
    [string]$OutFile = 'D:\CLI\_windows_high_nist\input-pack-result.txt'
)

$ErrorActionPreference = 'Stop'
$lines = New-Object System.Collections.Generic.List[string]
$utf8NoBom = New-Object System.Text.UTF8Encoding($false)

function Write-Line([string]$Text) {
    $lines.Add($Text)
    Write-Output $Text
}

function Get-FileSha256([string]$Path) {
    return (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}

function Remove-IfExists([string]$Path) {
    if (Test-Path -LiteralPath $Path) {
        Remove-Item -LiteralPath $Path -Recurse -Force
    }
}

function Update-TaskJson {
    param(
        [string]$TaskId,
        [string]$InputMode,
        [string]$Note
    )
    $tjPath = Join-Path $TasksRoot "$TaskId\task.json"
    if (-not (Test-Path -LiteralPath $tjPath)) {
        throw "task.json missing: $tjPath"
    }
    $tj = Get-Content -LiteralPath $tjPath -Raw -Encoding UTF8 | ConvertFrom-Json
    $tj | Add-Member -NotePropertyName 'input_mode' -NotePropertyValue $InputMode -Force
    if ($InputMode -eq 'inputs-present') {
        $tj | Add-Member -NotePropertyName 'inputs_manifest' -NotePropertyValue 'inputs-manifest.json' -Force
    }
    else {
        $tj | Add-Member -NotePropertyName 'inputs_manifest' -NotePropertyValue $null -Force
    }
    $tj | Add-Member -NotePropertyName 'input_note' -NotePropertyValue $Note -Force
    $tj | Add-Member -NotePropertyName 'app_mapping' -NotePropertyValue 'official /app -> C:\app (runner mirror); task cwd = workspace task dir' -Force
    [System.IO.File]::WriteAllText($tjPath, ($tj | ConvertTo-Json -Depth 10), $utf8NoBom)
}

function New-InputsManifest {
    param(
        [string]$TaskId,
        [string]$InputMode,
        [string]$Note
    )
    $taskDir = Join-Path $TasksRoot $TaskId
    $inputsDir = Join-Path $taskDir 'inputs'
    $files = New-Object System.Collections.ArrayList
    if (Test-Path -LiteralPath $inputsDir) {
        foreach ($f in (Get-ChildItem -LiteralPath $inputsDir -Recurse -File)) {
            $rel = $f.FullName.Substring($inputsDir.Length + 1)
            [void]$files.Add([ordered]@{
                path = $rel
                size = $f.Length
                sha256 = Get-FileSha256 $f.FullName
            })
        }
    }
    $manifest = [ordered]@{
        schema_version = '0.1.0'
        task_id = $TaskId
        input_mode = $InputMode
        input_note = $Note
        generated_at = (Get-Date -Format 'yyyy-MM-dd HH:mm:ss')
        files = $files
    }
    [System.IO.File]::WriteAllText(
        (Join-Path $taskDir 'inputs-manifest.json'),
        ($manifest | ConvertTo-Json -Depth 8),
        $utf8NoBom
    )
    Write-Line ("PACK task=$TaskId mode=$InputMode files=$($files.Count)")
}

function New-EmptyInputsManifest {
    param(
        [string]$TaskId,
        [string]$Note
    )
    $taskDir = Join-Path $TasksRoot $TaskId
    Remove-IfExists (Join-Path $taskDir 'inputs')
    $manifest = [ordered]@{
        schema_version = '0.1.0'
        task_id = $TaskId
        input_mode = 'no-input-diagnostic'
        input_note = $Note
        generated_at = (Get-Date -Format 'yyyy-MM-dd HH:mm:ss')
        files = @()
    }
    [System.IO.File]::WriteAllText(
        (Join-Path $taskDir 'inputs-manifest.json'),
        ($manifest | ConvertTo-Json -Depth 8),
        $utf8NoBom
    )
    Write-Line ("PACK task=$TaskId mode=no-input-diagnostic files=0")
}

New-Item -ItemType Directory -Path $CacheDir -Force | Out-Null

try {
    # ---------- gcode-to-text ----------
    $taskId = 'gcode-to-text'
    $inputs = Join-Path $TasksRoot "$taskId\inputs"
    Remove-IfExists $inputs
    New-Item -ItemType Directory -Path $inputs -Force | Out-Null
    $gz = Join-Path $OfficialRoot "$taskId\environment\text.gcode.gz"
    if (-not (Test-Path -LiteralPath $gz)) { throw "missing $gz" }
    $srcStream = [System.IO.File]::OpenRead($gz)
    $dstStream = [System.IO.File]::Create((Join-Path $inputs 'text.gcode'))
    $gzip = New-Object System.IO.Compression.GzipStream($srcStream, [System.IO.Compression.CompressionMode]::Decompress)
    try {
        $gzip.CopyTo($dstStream)
    }
    finally {
        $gzip.Dispose()
        $dstStream.Dispose()
        $srcStream.Dispose()
    }
    New-InputsManifest -TaskId $taskId -InputMode 'inputs-present' -Note 'official environment/text.gcode.gz decompressed -> text.gcode (cwd + C:\app)'
    Update-TaskJson -TaskId $taskId -InputMode 'inputs-present' -Note 'input file text.gcode supplied (gunzipped from official TB2.1 environment)'

    # ---------- make-doom-for-mips (network) ----------
    $taskId = 'make-doom-for-mips'
    $inputs = Join-Path $TasksRoot "$taskId\inputs"
    Remove-IfExists $inputs
    New-Item -ItemType Directory -Path $inputs -Force | Out-Null
    $repoCache = Join-Path $CacheDir 'doomgeneric'
    if (-not (Test-Path -LiteralPath (Join-Path $repoCache '.git'))) {
        Remove-IfExists $repoCache
        Write-Line 'DOOM_CLONE_START'
        & git clone --quiet https://github.com/ozkl/doomgeneric $repoCache
        if ($LASTEXITCODE -ne 0) { throw 'git clone doomgeneric failed' }
    }
    & git -C $repoCache reset --hard b94eba35b7cf4b2002857c7c625aeb24e99f979e
    if ($LASTEXITCODE -ne 0) { throw 'git reset doomgeneric failed' }
    $dstRepo = Join-Path $inputs 'doomgeneric'
    New-Item -ItemType Directory -Path $dstRepo -Force | Out-Null
    Get-ChildItem -LiteralPath $repoCache -Force | Where-Object { $_.Name -ne '.git' } |
        Copy-Item -Destination $dstRepo -Recurse -Force
    Write-Line ("DOOM_REPO_LAYOUT: " + (@(Get-ChildItem -LiteralPath $dstRepo -Directory | Select-Object -ExpandProperty Name) -join ','))
    $imgC = Join-Path $OfficialRoot "$taskId\environment\task-deps\doomgeneric_img.c"
    if (-not (Test-Path -LiteralPath $imgC)) { throw "missing $imgC" }
    $imgDst = Join-Path $dstRepo 'doomgeneric\doomgeneric_img.c'
    Copy-Item -LiteralPath $imgC -Destination $imgDst -Force
    Copy-Item -LiteralPath (Join-Path $OfficialRoot "$taskId\environment\task-deps\vm.js") -Destination (Join-Path $inputs 'vm.js') -Force
    $wadOut = Join-Path $inputs 'doom.wad'
    # The official ibiblio URL is dead (404, 2026-09-03); canonical Doom 1.9
    # shareware doom1.wad is cached from Doom-Utils/shareware-collection
    # (sha256 1d7d43be501e67d927e415e0b8f3e29c3bf33075e859721816f652a526cac771).
    $wadSrc = Join-Path $CacheDir 'shareware-collection\Doom 1.9\doom1.wad'
    if (-not (Test-Path -LiteralPath $wadSrc)) {
        $wadSrc = Join-Path $CacheDir 'vanilla-mocha-doom\wads\doom1.wad'
    }
    if (-not (Test-Path -LiteralPath $wadSrc)) { throw 'doom1.wad cache missing' }
    Copy-Item -LiteralPath $wadSrc -Destination $wadOut -Force
    Write-Line ("DOOM_WAD size=" + (Get-Item -LiteralPath $wadOut).Length)
    New-InputsManifest -TaskId $taskId -InputMode 'inputs-present' -Note 'doomgeneric pinned b94eba35 + task-deps doomgeneric_img.c/vm.js + doom.wad (mirror of official /app)'
    Update-TaskJson -TaskId $taskId -InputMode 'inputs-present' -Note 'doomgeneric source tree + doomgeneric_img.c + vm.js + doom.wad supplied under C:\app (mirror of official /app)'

    # ---------- path-tracing (render with host gcc) ----------
    $taskId = 'path-tracing'
    $inputs = Join-Path $TasksRoot "$taskId\inputs"
    Remove-IfExists $inputs
    New-Item -ItemType Directory -Path $inputs -Force | Out-Null
    $renderDir = Join-Path $CacheDir 'path-tracing-render'
    New-Item -ItemType Directory -Path $renderDir -Force | Out-Null
    Copy-Item -LiteralPath (Join-Path $OfficialRoot "$taskId\environment\orig.c") -Destination (Join-Path $renderDir 'orig.c') -Force
    $exe = Join-Path $renderDir 'orig-render.exe'
    $gcc = (Get-Command gcc -ErrorAction Stop).Source
    & $gcc -O2 -o $exe (Join-Path $renderDir 'orig.c') -lm
    if ($LASTEXITCODE -ne 0) { throw 'gcc orig.c failed' }
    Push-Location $renderDir
    try {
        $renderOut = Join-Path $renderDir 'render.stdout.txt'
        $renderErr = Join-Path $renderDir 'render.stderr.txt'
        $rp = Start-Process -FilePath $exe -WorkingDirectory $renderDir -PassThru -Wait -WindowStyle Hidden `
            -RedirectStandardOutput $renderOut -RedirectStandardError $renderErr
        if ($rp.ExitCode -ne 0) {
            Get-Content -LiteralPath $renderErr -Encoding UTF8 -ErrorAction SilentlyContinue |
                ForEach-Object { Write-Line ("RENDER_STDERR: " + $_) }
            throw "orig-render.exe failed (exit=$($rp.ExitCode))"
        }
    }
    finally {
        Pop-Location
    }
    $ppm = Join-Path $renderDir 'image.ppm'
    if (-not (Test-Path -LiteralPath $ppm)) { throw 'image.ppm not produced' }
    Copy-Item -LiteralPath $ppm -Destination (Join-Path $inputs 'image.ppm') -Force
    $head = Get-Content -LiteralPath (Join-Path $inputs 'image.ppm') -TotalCount 2
    Write-Line ("PATH_TRACING_PPM_HEAD: " + ($head -join ' | '))
    New-InputsManifest -TaskId $taskId -InputMode 'inputs-present' -Note 'image.ppm rendered from official environment/orig.c (host mingw gcc -O2)'
    Update-TaskJson -TaskId $taskId -InputMode 'inputs-present' -Note 'reference image.ppm supplied (rendered from official orig.c; orig.c itself not shipped, matching official env)'

    # ---------- model-extraction-relu-logits ----------
    $taskId = 'model-extraction-relu-logits'
    $inputs = Join-Path $TasksRoot "$taskId\inputs"
    Remove-IfExists $inputs
    New-Item -ItemType Directory -Path $inputs -Force | Out-Null
    Copy-Item -LiteralPath (Join-Path $OfficialRoot "$taskId\environment\forward.py") -Destination (Join-Path $inputs 'forward.py') -Force
    New-InputsManifest -TaskId $taskId -InputMode 'inputs-present' -Note 'official environment/forward.py (queryable oracle)'
    Update-TaskJson -TaskId $taskId -InputMode 'inputs-present' -Note 'forward.py oracle supplied (cwd + C:\app)'

    # ---------- dna-insert ----------
    $taskId = 'dna-insert'
    $inputs = Join-Path $TasksRoot "$taskId\inputs"
    Remove-IfExists $inputs
    New-Item -ItemType Directory -Path $inputs -Force | Out-Null
    Copy-Item -LiteralPath (Join-Path $OfficialRoot "$taskId\environment\sequences.fasta") -Destination (Join-Path $inputs 'sequences.fasta') -Force
    New-InputsManifest -TaskId $taskId -InputMode 'inputs-present' -Note 'official environment/sequences.fasta'
    Update-TaskJson -TaskId $taskId -InputMode 'inputs-present' -Note 'sequences.fasta supplied (cwd + C:\app)'

    # ---------- no-input-diagnostic tasks ----------
    New-EmptyInputsManifest -TaskId 'mteb-leaderboard' -Note 'retrieval task with no /app input files; requires general web egress which the high-nist allowlist blocks'
    Update-TaskJson -TaskId 'mteb-leaderboard' -InputMode 'no-input-diagnostic' -Note 'no task input files by design (network retrieval); wall egress = DeepSeek allowlist only'
    New-EmptyInputsManifest -TaskId 'train-fasttext' -Note 'official /app/data parquet train set not fetched (~sizeable) and fasttext toolchain absent in guest; deferred to env-provision follow-up'
    Update-TaskJson -TaskId 'train-fasttext' -InputMode 'no-input-diagnostic' -Note 'data/ folder absent: yelp parquet fetch + fasttext install deferred (F4 env provisioning)'
    New-EmptyInputsManifest -TaskId 'adaptive-rejection-sampler' -Note 'write-code task; official /protected artifact is verifier-side only; R toolchain absent in guest'
    Update-TaskJson -TaskId 'adaptive-rejection-sampler' -InputMode 'no-input-diagnostic' -Note 'no agent-facing input files; R toolchain absent (F4 env provisioning)'
    New-EmptyInputsManifest -TaskId 'filter-js-from-html' -Note 'write-code task (filter.py); verifier supplies HTML at test time'
    Update-TaskJson -TaskId 'filter-js-from-html' -InputMode 'no-input-diagnostic' -Note 'no agent-facing input files; code-writing task'

    # ---------- manifest refresh ----------
    $manifestPath = Join-Path $TasksRoot 'manifest.json'
    $m = Get-Content -LiteralPath $manifestPath -Raw -Encoding UTF8 | ConvertFrom-Json
    $modeById = @{}
    foreach ($t in @('gcode-to-text', 'make-doom-for-mips', 'mteb-leaderboard', 'path-tracing', 'train-fasttext', 'adaptive-rejection-sampler', 'filter-js-from-html', 'model-extraction-relu-logits', 'dna-insert')) {
        $tj = Get-Content -LiteralPath (Join-Path $TasksRoot "$t\task.json") -Raw -Encoding UTF8 | ConvertFrom-Json
        $modeById[$t] = $tj.input_mode
    }
    foreach ($entry in $m.tasks) {
        $entry | Add-Member -NotePropertyName 'input_mode' -NotePropertyValue $modeById[[string]$entry.id] -Force
    }
    $m | Add-Member -NotePropertyName 'input_loading' -NotePropertyValue 'F2 2026-09-03: inputs-present tasks carry taskDir\inputs (mirrored by runner into workdir + C:\app); no-input-diagnostic tasks run without files and are recorded here/task.json' -Force
    [System.IO.File]::WriteAllText($manifestPath, ($m | ConvertTo-Json -Depth 10), $utf8NoBom)
    Write-Line 'PACK_MANIFEST_UPDATED=1'
    Write-Line 'PACK_ALL_DONE=1'
}
catch {
    Write-Line ("ERROR: " + $_.Exception.Message)
    Write-Line 'PACK_ALL_DONE=0'
}

$lines | Set-Content -LiteralPath $OutFile -Encoding utf8
Write-Output "resultFile=$OutFile"
if (@($lines | Where-Object { $_ -match '^PACK_ALL_DONE=1$' }).Count -gt 0) {
    exit 0
}
exit 1
