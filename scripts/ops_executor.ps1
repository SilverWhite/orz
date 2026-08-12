#requires -Version 5.1
<#
Structured Operation Protocol v0.1 -- Windows PowerShell reference executor.

Reads one operation JSON from -OpFile or stdin, validates it strictly, applies
the mechanical safety policy (scope, cache classification, recycle-bin
capacity, trash-by-default), executes or rejects, and appends one audit JSONL
record.

Exit codes:
  0  executed or dry-run plan produced
  1  invalid input or execution error
  2  policy rejection (no mutation performed)
#>

[CmdletBinding()]
param(
    [string]$OpFile,
    [string[]]$AllowRoot,
    [string]$AuditLog
)

$ErrorActionPreference = 'Stop'
try { [Console]::OutputEncoding = New-Object System.Text.UTF8Encoding($false) } catch { }
if (-not ('OpsNative' -as [type])) {
    Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
using System.Text;
public static class OpsNative {
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    public static extern IntPtr CreateFileW(string lpFileName, uint dwDesiredAccess, uint dwShareMode, IntPtr lpSecurityAttributes, uint dwCreationDisposition, uint dwFlagsAndAttributes, IntPtr hTemplateFile);
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    public static extern uint GetFinalPathNameByHandleW(IntPtr hFile, StringBuilder lpszFilePath, uint cchFilePath, uint dwFlags);
    [DllImport("kernel32.dll", SetLastError = true)]
    public static extern bool CloseHandle(IntPtr hObject);
    public static string GetFinalPath(string path) {
        IntPtr h = CreateFileW(path, 0, 1 | 2 | 4, IntPtr.Zero, 3, 0x02000000, IntPtr.Zero);
        if (h == new IntPtr(-1)) return null;
        try {
            var sb = new StringBuilder(512);
            uint n = GetFinalPathNameByHandleW(h, sb, 512, 0);
            if (n == 0 || n > 512) return null;
            string r = sb.ToString();
            if (r.StartsWith(@"\\?\UNC\", StringComparison.OrdinalIgnoreCase)) {
                return @"\\" + r.Substring(8);
            }
            if (r.StartsWith(@"\\?\", StringComparison.OrdinalIgnoreCase)) {
                return r.Substring(4);
            }
            return r;
        } finally { CloseHandle(h); }
    }
}
"@
}

$script:AuditLog = $AuditLog
if (-not $script:AuditLog) { $script:AuditLog = $env:OPS_AUDIT_LOG }
if (-not $script:AuditLog) { $script:AuditLog = Join-Path (Get-Location) 'ops-audit.jsonl' }

$script:AllowRoots = @()
if ($AllowRoot -and $AllowRoot.Count -gt 0) {
    $script:AllowRoots = @($AllowRoot)
} elseif ($env:OPS_ALLOW_ROOTS) {
    $script:AllowRoots = @($env:OPS_ALLOW_ROOTS -split ';' | Where-Object { $_ -and $_.Trim() })
}
$script:AllowRoots = @($script:AllowRoots | ForEach-Object { [System.IO.Path]::GetFullPath($_) })

$script:Ops = @{
    'file.read'   = @{ Targets = $true;  Fields = @('max_bytes') }
    'file.write'  = @{ Targets = $true;  Fields = @('content') }
    'file.move'   = @{ Targets = $true;  Fields = @('dest') }
    'file.copy'   = @{ Targets = $true;  Fields = @('dest') }
    'file.delete' = @{ Targets = $true;  Fields = @('dry_run') }
    'dir.list'    = @{ Targets = $true;  Fields = @('depth', 'max_entries') }
    'process.run' = @{ Targets = $false; Fields = @('exe', 'args', 'cwd', 'timeout_ms') }
    'env.info'    = @{ Targets = $false; Fields = @() }
}

function ConvertTo-NormPath([string]$Path) {
    return [System.IO.Path]::GetFullPath($Path)
}

function Resolve-RealPath([string]$Path) {
    # Resolve the full reparse chain of the deepest existing ancestor, then
    # re-append the non-existing tail. Mirrors os.path.realpath semantics.
    $full = ConvertTo-NormPath $Path
    $tail = New-Object System.Collections.Generic.List[string]
    $probe = $full
    while (-not (Test-Path -LiteralPath $probe)) {
        $leaf = Split-Path -Leaf $probe
        if (-not $leaf -or $leaf -eq $probe) { break }
        $tail.Insert(0, $leaf)
        $parent = Split-Path -Parent $probe
        if (-not $parent -or $parent -eq $probe) { break }
        $probe = $parent
    }
    if (Test-Path -LiteralPath $probe) {
        try {
            $resolved = [OpsNative]::GetFinalPath($probe)
            if ($resolved) {
                $full = $resolved
                foreach ($t in $tail) { $full = Join-Path $full $t }
            }
        } catch { }
    }
    return ConvertTo-NormPath $full
}

function Resolve-Scoped([string]$Path) {
    $full = Resolve-RealPath $Path
    foreach ($root in $script:AllowRoots) {
        $r = Resolve-RealPath $root
        $cmp = [StringComparison]::OrdinalIgnoreCase
        if ($full.Equals($r, $cmp) -or $full.StartsWith($r.TrimEnd('\') + '\', $cmp)) {
            return $full
        }
    }
    throw 'OPS-POLICY: target outside allow roots: ' + $full
}

function Test-InsideRoots([string]$Path) {
    $full = Resolve-RealPath $Path
    foreach ($root in $script:AllowRoots) {
        $r = Resolve-RealPath $root
        $cmp = [StringComparison]::OrdinalIgnoreCase
        if ($full.Equals($r, $cmp) -or $full.StartsWith($r.TrimEnd('\') + '\', $cmp)) {
            return $true
        }
    }
    return $false
}

function ConvertFrom-OpsBytes([byte[]]$Bytes) {
    # Fixed decode chain: BOM strip -> UTF-8 strict -> GB18030 -> UTF-8 lossy.
    $data = $Bytes
    $label = 'utf-8'
    if ($data.Length -ge 3 -and $data[0] -eq 0xEF -and $data[1] -eq 0xBB -and $data[2] -eq 0xBF) {
        if ($data.Length -eq 3) { $data = [byte[]]@() } else { $data = $data[3..($data.Length - 1)] }
        $label = 'utf-8-sig'
    }
    try {
        $strict = New-Object System.Text.UTF8Encoding($false, $true)
        return @{ Text = $strict.GetString($data); Encoding = $label }
    } catch { }
    try {
        $gb = [System.Text.Encoding]::GetEncoding(54936)
        return @{ Text = $gb.GetString($data); Encoding = 'gb18030' }
    } catch { }
    $lossy = New-Object System.Text.UTF8Encoding($false, $false)
    return @{ Text = $lossy.GetString($data); Encoding = 'utf-8-lossy' }
}

function Assert-NotSystemRoot([string]$Path) {
    $full = (ConvertTo-NormPath $Path).TrimEnd('\')
    if ($full -match '^[A-Za-z]:$' -or $full -eq '\') {
        throw 'OPS-POLICY: refusing to target a filesystem root: ' + $Path
    }
}

function Assert-NotAllowRoot([string]$Path) {
    $full = ConvertTo-NormPath $Path
    foreach ($root in $script:AllowRoots) {
        if ($full.Equals((ConvertTo-NormPath $root), [StringComparison]::OrdinalIgnoreCase)) {
            throw 'OPS-POLICY: refusing to target an allow root itself: ' + $Path
        }
    }
}

function Get-ProtectedSet {
    $set = New-Object 'System.Collections.Generic.HashSet[string]' ([StringComparer]::OrdinalIgnoreCase)
    try { [void]$set.Add((Resolve-RealPath $PSCommandPath)) } catch { }
    try { [void]$set.Add((Resolve-RealPath $script:AuditLog)) } catch { }
    foreach ($p in @($env:OPS_PROTECTED -split ';')) {
        if ($p.Trim()) {
            try { [void]$set.Add((Resolve-RealPath $p.Trim())) } catch { }
        }
    }
    return $set
}

function Test-Protected([string]$Path) {
    if ((Split-Path -Leaf $Path) -eq '.ops.json') { return $true }
    return (Get-ProtectedSet).Contains((Resolve-RealPath $Path))
}

function Assert-NotProtected([string]$Path) {
    if (Test-Protected $Path) {
        throw 'OPS-POLICY: refusing to mutate a protected file: ' + $Path
    }
}

function Get-ProcessAllowlist {
    $out = New-Object 'System.Collections.Generic.HashSet[string]' ([StringComparer]::OrdinalIgnoreCase)
    foreach ($p in @($env:OPS_PROCESS_ALLOW -split ';')) {
        if ($p.Trim()) { [void]$out.Add($p.Trim()) }
    }
    return $out
}

function Test-OpsGlob([string]$Pattern, [string]$Path) {
    $p = $Path.Replace('\', '/')
    $pat = $Pattern.Replace('\', '/').Replace('**', '*')
    if (-not $pat.StartsWith('/')) { $pat = '*/' + $pat }
    return ($p -like $pat)
}

function Find-OpsConfig([string]$Path) {
    $cur = Split-Path -Parent (Resolve-Scoped $Path)
    for ($i = 0; $i -lt 64; $i++) {
        $cfgPath = Join-Path $cur '.ops.json'
        if (Test-Path -LiteralPath $cfgPath -PathType Leaf) {
            try {
                $data = Get-Content -LiteralPath $cfgPath -Raw -Encoding UTF8 | ConvertFrom-Json
                if ($data -is [pscustomobject]) {
                    return @{ Config = $data; Malformed = $false }
                }
                return @{ Config = $null; Malformed = $true }
            } catch {
                return @{ Config = $null; Malformed = $true }
            }
        }
        $hitRoot = $false
        foreach ($root in $script:AllowRoots) {
            if ((ConvertTo-NormPath $cur).Equals((ConvertTo-NormPath $root), [StringComparison]::OrdinalIgnoreCase)) {
                $hitRoot = $true
                break
            }
        }
        if ($hitRoot) { break }
        $parent = Split-Path -Parent $cur
        if (-not $parent -or $parent -eq $cur) { break }
        $cur = $parent
    }
    return @{ Config = $null; Malformed = $false }
}

function Get-OpsClass([string]$Path) {
    $full = Resolve-Scoped $Path
    $found = Find-OpsConfig $full
    if ($found.Malformed) {
        return [pscustomobject]@{ Class = 'protected'; Evidence = 'malformed .ops.json (fail-closed)' }
    }

    # Ancestor chain: target + parents up to (excluding) the allow root.
    $chain = New-Object System.Collections.Generic.List[string]
    $chain.Add($full)
    $cur = Split-Path -Parent $full
    while ($cur) {
        $hitRoot = $false
        foreach ($root in $script:AllowRoots) {
            if ((Resolve-RealPath $cur).Equals((Resolve-RealPath $root), [StringComparison]::OrdinalIgnoreCase)) {
                $hitRoot = $true
                break
            }
        }
        if ($hitRoot) { break }
        $chain.Add($cur)
        $parent = Split-Path -Parent $cur
        if (-not $parent -or $parent -eq $cur) { break }
        $cur = $parent
    }

    $cfg = $found.Config
    if ($cfg) {
        foreach ($p in $chain) {
            foreach ($pat in @($cfg.protected)) {
                if (Test-OpsGlob ([string]$pat) $p) {
                    return [pscustomobject]@{ Class = 'protected'; Evidence = ('project config protected: {0}' -f $pat) }
                }
            }
        }
        foreach ($p in $chain) {
            foreach ($pat in @($cfg.disposable)) {
                if (Test-OpsGlob ([string]$pat) $p) {
                    return [pscustomobject]@{ Class = 'cache'; Evidence = ('project config disposable: {0}' -f $pat) }
                }
            }
        }
    }

    foreach ($p in $chain) {
        $name = Split-Path -Leaf $p
        $rel = $name
        foreach ($root in $script:AllowRoots) {
            $r = (Resolve-RealPath $root).TrimEnd('\')
            if ($p.StartsWith($r + '\', [StringComparison]::OrdinalIgnoreCase)) {
                $rel = $p.Substring($r.Length).TrimStart('\')
                break
            }
        }
        if ($name -eq '__pycache__' -and (Test-Path -LiteralPath $p -PathType Container)) {
            $all = @(Get-ChildItem -LiteralPath $p -Recurse -Force -File -ErrorAction SilentlyContinue)
            if (@($all | Where-Object { $_.Name -notmatch '\.py[co]$' }).Count -eq 0) {
                return [pscustomobject]@{ Class = 'cache'; Evidence = ('builtin: {0} __pycache__ contains only .pyc/.pyo' -f $rel) }
            }
        }
        if ($name -eq 'target' -and (Test-Path -LiteralPath $p -PathType Container)) {
            $markers = @('debug', 'release', '.fingerprint', 'CACHEDIR.TAG') |
                Where-Object { Test-Path -LiteralPath (Join-Path $p $_) }
            if ($markers.Count -gt 0) {
                return [pscustomobject]@{ Class = 'cache'; Evidence = ('builtin: {0} target/{1}' -f $rel, ($markers -join ',')) }
            }
        }
        if ($name -eq 'node_modules' -and (Test-Path -LiteralPath $p -PathType Container)) {
            if ((Test-Path -LiteralPath (Join-Path $p '.package-lock.json')) -or
                (Test-Path -LiteralPath (Join-Path $p '.bin'))) {
                return [pscustomobject]@{ Class = 'cache'; Evidence = ('builtin: {0} node_modules/.package-lock.json or .bin' -f $rel) }
            }
            $parent = Split-Path -Parent $p
            if ((Test-Path -LiteralPath (Join-Path $parent 'package.json')) -or
                (Test-Path -LiteralPath (Join-Path $parent 'package-lock.json'))) {
                return [pscustomobject]@{ Class = 'cache'; Evidence = ('builtin: {0} node_modules with package manifests' -f $rel) }
            }
        }
        if ($p -match '\\\.cargo\\registry($|\\)' -or $p -match '/\.cargo/registry($|/)') {
            return [pscustomobject]@{ Class = 'cache'; Evidence = ('builtin: {0} .cargo/registry' -f $rel) }
        }
        if ($p -match '\\\.gradle\\caches($|\\)' -or $p -match '/\.gradle/caches($|/)') {
            return [pscustomobject]@{ Class = 'cache'; Evidence = ('builtin: {0} .gradle/caches' -f $rel) }
        }
        if ((Test-Path -LiteralPath (Join-Path $p 'CACHEDIR.TAG')) -and (Test-Path -LiteralPath $p -PathType Container)) {
            return [pscustomobject]@{ Class = 'cache'; Evidence = ('builtin: {0} CACHEDIR.TAG' -f $rel) }
        }
    }
    return [pscustomobject]@{ Class = 'unknown'; Evidence = 'no classification rule matched' }
}

function Get-OpsSize([string]$Path) {
    $item = Get-Item -LiteralPath $Path -Force -ErrorAction Stop
    if (-not $item.PSIsContainer) {
        return [pscustomobject]@{ Bytes = [int64]$item.Length; Files = 1; Dirs = 0 }
    }
    $sum = 0L
    $files = 0
    $dirs = 0
    $stack = New-Object System.Collections.Generic.Stack[string]
    $stack.Push($Path)
    while ($stack.Count -gt 0) {
        $dir = $stack.Pop()
        foreach ($e in Get-ChildItem -LiteralPath $dir -Force -ErrorAction SilentlyContinue) {
            if ($e.PSIsContainer) {
                $dirs++
                if (-not ($e.Attributes -band [IO.FileAttributes]::ReparsePoint)) {
                    $stack.Push($e.FullName)
                }
            } else {
                $files++
                $sum += $e.Length
            }
        }
    }
    return [pscustomobject]@{ Bytes = $sum; Files = $files; Dirs = $dirs }
}

function Get-RecycleCapacity([string]$Path) {
    $drive = Split-Path -Qualifier $Path
    if (-not $drive) { $drive = (Get-Location).Drive.Name + ':' }
    $cap = $null
    try {
        $disk = Get-CimInstance Win32_LogicalDisk -Filter ("DeviceID='{0}'" -f $drive)
        if ($disk) { $cap = [int64]($disk.Size * 0.10) }
    } catch { }
    try {
        $vol = Get-Volume -DriveLetter $drive.TrimEnd(':') -ErrorAction Stop
        if ($vol.UniqueId -match '\{([0-9A-Fa-f-]+)\}') {
            $key = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\BitBucket\Volume\{' + $matches[1] + '}'
            if (Test-Path $key) {
                $mb = (Get-ItemProperty -LiteralPath $key -Name MaxCapacity -ErrorAction SilentlyContinue).MaxCapacity
                if ($mb) { return [int64]$mb * 1MB }
            }
        }
    } catch { }
    if ($cap) { return $cap }
    return [int64]5GB
}

function Invoke-OpsDelete([string]$Path, [string]$Mode) {
    Assert-NotSystemRoot $Path
    Assert-NotAllowRoot $Path
    if ($Mode -eq 'trash') {
        Add-Type -AssemblyName Microsoft.VisualBasic
        $item = Get-Item -LiteralPath $Path -Force
        if ($item.PSIsContainer) {
            [Microsoft.VisualBasic.FileIO.FileSystem]::DeleteDirectory($Path, 'OnlyErrorDialogs', 'SendToRecycleBin')
        } else {
            [Microsoft.VisualBasic.FileIO.FileSystem]::DeleteFile($Path, 'OnlyErrorDialogs', 'SendToRecycleBin')
        }
    } else {
        Remove-Item -LiteralPath $Path -Recurse -Force
    }
}

function Resolve-DeleteTargets([object[]]$Targets) {
    $out = @()
    foreach ($t in $Targets) {
        $full = Resolve-Scoped ([string]$t)
        Assert-NotSystemRoot $full
        Assert-NotAllowRoot $full
        Assert-NotProtected $full
        $cls = Get-OpsClass $full
        $sz = Get-OpsSize $full
        $out += [pscustomobject]@{
            input      = $t
            absolute   = $full
            class      = $cls.Class
            evidence   = $cls.Evidence
            size_bytes = $sz.Bytes
            files      = $sz.Files
            dirs       = $sz.Dirs
        }
    }
    return ,$out
}

function Get-OpsPolicy([object[]]$Resolved) {
    $classes = @($Resolved | ForEach-Object { $_.class } | Select-Object -Unique)
    $total = [int64]($Resolved | Measure-Object -Property size_bytes -Sum).Sum
    $cap = [int64]($Resolved | ForEach-Object { Get-RecycleCapacity $_.absolute } | Measure-Object -Maximum).Maximum
    if ($classes -contains 'protected') {
        $ev = @($Resolved | Where-Object { $_.class -eq 'protected' } | Select-Object -First 1).evidence
        if (-not $ev) { $ev = 'protected pattern matched' }
        return @{ decision = 'reject'; mode = $null; reason = ('protected: {0}; refuse deletion' -f $ev) }
    }
    if ($classes.Count -eq 1 -and $classes -contains 'cache') {
        return @{ decision = 'permit'; mode = 'permanent'; reason = 'all targets classified as cache; evidence recorded per target' }
    }
    if ($total -le $cap) {
        return @{ decision = 'permit'; mode = 'trash'; reason = ('non-cache within capacity ({0} <= {1})' -f $total, $cap) }
    }
    return @{ decision = 'reject'; mode = $null; reason = ('non-cache over capacity ({0} > {1}); permanent delete refused' -f $total, $cap) }
}

function Read-File([pscustomobject]$Raw) {
    $maxBytes = [int64]10MB
    if ($Raw.PSObject.Properties['max_bytes'] -and $Raw.max_bytes) { $maxBytes = [int64]$Raw.max_bytes }
    $out = @()
    $total = [int64]0
    foreach ($t in @($Raw.targets)) {
        $full = Resolve-Scoped ([string]$t)
        if (-not (Test-Path -LiteralPath $full -PathType Leaf)) { throw "not a file: $full" }
        $length = (Get-Item -LiteralPath $full -Force).Length
        if ($length -gt $maxBytes) { throw "file exceeds max_bytes: $full" }
        $total += $length
        if ($total -gt [int64]64MB) { throw 'aggregate read size exceeds 67108864 bytes' }
        $bytes = [System.IO.File]::ReadAllBytes($full)
        $decoded = ConvertFrom-OpsBytes $bytes
        $out += @{ path = $full; content = $decoded.Text; encoding = $decoded.Encoding }
    }
    return @{ files = $out }
}

function Write-File([pscustomobject]$Raw) {
    $full = Resolve-Scoped ([string]$Raw.targets[0])
    Assert-NotProtected $full
    $parent = Split-Path -Parent $full
    if (-not $parent) { $parent = (Get-Location).Path }
    if (-not (Test-Path -LiteralPath $parent -PathType Container)) {
        New-Item -ItemType Directory -Path $parent -Force | Out-Null
    }
    $tmp = Join-Path $parent ('.ops-write-' + [guid]::NewGuid().ToString('N') + '.tmp')
    try {
        [System.IO.File]::WriteAllText($tmp, [string]$Raw.content, (New-Object System.Text.UTF8Encoding($false)))
        Move-Item -LiteralPath $tmp -Destination $full -Force
    } finally {
        if (Test-Path -LiteralPath $tmp) { Remove-Item -LiteralPath $tmp -Force -ErrorAction SilentlyContinue }
    }
    return @{ path = $full; bytes = [System.Text.Encoding]::UTF8.GetByteCount([string]$Raw.content) }
}

function Move-Copy([pscustomobject]$Raw, [bool]$CopyMode) {
    $src = Resolve-Scoped ([string]$Raw.targets[0])
    $dest = Resolve-Scoped ([string]$Raw.dest)
    Assert-NotProtected $src
    Assert-NotProtected $dest
    if (-not (Test-Path -LiteralPath $src)) { throw "source does not exist: $src" }
    if (Test-Path -LiteralPath $dest -PathType Container) {
        $dest = Join-Path $dest (Split-Path -Leaf $src)
    }
    if ($CopyMode) {
        if ((Get-Item -LiteralPath $src).PSIsContainer) {
            Copy-Item -LiteralPath $src -Destination $dest -Recurse -Force
        } else {
            Copy-Item -LiteralPath $src -Destination $dest -Force
        }
    } else {
        Move-Item -LiteralPath $src -Destination $dest -Force
    }
    return @{ source = $src; dest = $dest }
}

function Get-DirList([pscustomobject]$Raw) {
    $depth = 1
    $maxEntries = 10000
    if ($Raw.PSObject.Properties['depth'] -and $Raw.depth) { $depth = [int]$Raw.depth }
    if ($Raw.PSObject.Properties['max_entries'] -and $Raw.max_entries) { $maxEntries = [int]$Raw.max_entries }
    $out = @()
    foreach ($t in @($Raw.targets)) {
        $full = Resolve-Scoped ([string]$t)
        if (-not (Test-Path -LiteralPath $full -PathType Container)) { throw "not a directory: $full" }
        $script:OpsEntries = New-Object System.Collections.Generic.List[object]
        $script:OpsMaxEntries = $maxEntries
        $walk = {
            param($d, $level)
            foreach ($e in Get-ChildItem -LiteralPath $d -Force -ErrorAction SilentlyContinue) {
                if ($script:OpsEntries.Count -ge $script:OpsMaxEntries) { break }
                $isDir = $e.PSIsContainer
                $sz = 0
                if (-not $isDir) { $sz = $e.Length }
                $script:OpsEntries.Add([pscustomobject]@{
                    name = $e.Name
                    type = $(if ($isDir) { 'dir' } else { 'file' })
                    size = $sz
                })
                $isLink = ($e.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0
                if ($isDir -and -not $isLink -and $level -lt $depth) { & $walk $e.FullName ($level + 1) }
            }
        }
        & $walk $full 1
        $dirResult = @{
            path      = $full
            entries   = $script:OpsEntries.ToArray()
            truncated = ($script:OpsEntries.Count -ge $maxEntries)
        }
        $out += $dirResult
    }
    return @{ dirs = $out }
}

function ConvertTo-CommandLine([string[]]$Items) {
    # Fixed list2cmdline-style quoting for CreateProcess (Windows PowerShell
    # 5.1 / .NET Framework has no ProcessStartInfo.ArgumentList).
    $out = New-Object System.Collections.Generic.List[string]
    foreach ($item in $Items) {
        $s = [string]$item
        if ($s.Length -eq 0) {
            $out.Add('""')
            continue
        }
        if ($s -notmatch '[\s"]') {
            $out.Add($s)
            continue
        }
        $sb = New-Object System.Text.StringBuilder
        [void]$sb.Append('"')
        $i = 0
        while ($i -lt $s.Length) {
            $backslashes = 0
            while ($i -lt $s.Length -and $s[$i] -eq '\') { $backslashes++; $i++ }
            if ($i -ge $s.Length) {
                [void]$sb.Append('\' * ($backslashes * 2))
            } elseif ($s[$i] -eq '"') {
                [void]$sb.Append('\' * ($backslashes * 2 + 1))
                [void]$sb.Append('"')
                $i++
            } else {
                [void]$sb.Append('\' * $backslashes)
                [void]$sb.Append($s[$i])
                $i++
            }
        }
        [void]$sb.Append('"')
        $out.Add($sb.ToString())
    }
    return ($out -join ' ')
}

function Invoke-OpsProcess([string]$Exe, [string[]]$ArgList, [string]$Cwd, [int]$TimeoutMs) {
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = $Exe
    $psi.UseShellExecute = $false
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    try {
        $psi.StandardOutputEncoding = New-Object System.Text.UTF8Encoding($false)
        $psi.StandardErrorEncoding = New-Object System.Text.UTF8Encoding($false)
    } catch { }
    if ($Cwd) { $psi.WorkingDirectory = $Cwd }
    $psi.Arguments = ConvertTo-CommandLine $ArgList
    $p = [System.Diagnostics.Process]::Start($psi)
    $outTask = $p.StandardOutput.ReadToEndAsync()
    $errTask = $p.StandardError.ReadToEndAsync()
    if (-not $p.WaitForExit($TimeoutMs)) {
        try { $p.Kill() } catch { }
        throw ('process timed out after {0}ms' -f $TimeoutMs)
    }
    return @{
        exit_code       = $p.ExitCode
        stdout_tail     = $outTask.Result
        stderr_tail     = $errTask.Result
        stdout_encoding = 'utf-8'
        stderr_encoding = 'utf-8'
    }
}

function Get-EnvInfo {
    return [pscustomobject]@{
        host          = 'windows'
        platform      = [System.Environment]::OSVersion.VersionString
        shell         = 'powershell'
        shell_version = $PSVersionTable.PSVersion.ToString()
        path_style    = 'windows'
        eol           = 'crlf'
        encoding      = 'utf-8'
        cwd           = (Get-Location).Path
        allow_roots   = $script:AllowRoots
        audit_log     = $script:AuditLog
    }
}

function Write-OpsAudit([pscustomobject]$Entry) {
    $parent = Split-Path -Parent $script:AuditLog
    if ($parent -and -not (Test-Path -LiteralPath $parent -PathType Container)) {
        New-Item -ItemType Directory -Path $parent -Force | Out-Null
    }
    $line = $Entry | ConvertTo-Json -Compress -Depth 12
    [System.IO.File]::AppendAllText($script:AuditLog, $line + [Environment]::NewLine,
        (New-Object System.Text.UTF8Encoding($false)))
}

function Validate-Op([object]$Raw) {
    if ($Raw -isnot [pscustomobject]) { throw 'envelope must be a JSON object' }
    foreach ($f in @('proto', 'op', 'targets')) {
        if (-not $Raw.PSObject.Properties[$f]) { throw "missing required field: $f" }
    }
    if ([int]$Raw.proto -ne 1) { throw 'unsupported proto' }
    if ($Raw.op -isnot [string]) { throw 'op must be a string' }
    $op = [string]$Raw.op
    if (-not $script:Ops.ContainsKey($op)) { throw "unknown op: $op" }
    $spec = $script:Ops[$op]
    $common = @('proto', 'id', 'op', 'targets', 'reason')
    $unknown = @($Raw.PSObject.Properties.Name | Where-Object { $_ -notin $common -and $_ -notin $spec.Fields })
    if ($unknown.Count -gt 0) { throw ('unknown fields: {0}' -f ($unknown -join ', ')) }

    if ($Raw.PSObject.Properties['id']) {
        $idVal = [string]$Raw.id
        if ($idVal -notmatch '^[A-Za-z0-9._-]{1,80}$') { throw 'invalid id' }
    }
    if ($Raw.PSObject.Properties['reason']) {
        $reasonVal = [string]$Raw.reason
        if ($reasonVal.Length -lt 1 -or $reasonVal.Length -gt 500) { throw 'reason must be a string of 1..500 chars' }
    }
    if ($Raw.targets -isnot [array]) { throw 'targets must be an array' }
    $targets = @($Raw.targets)
    if ($targets.Count -gt 1000) { throw 'targets exceeds 1000 items' }
    if ($spec.Targets -and $targets.Count -eq 0) { throw 'targets must be a non-empty array' }
    foreach ($t in $targets) {
        if ($t -isnot [string]) { throw 'targets entries must be strings' }
        if ([string]::IsNullOrWhiteSpace($t)) { throw 'targets entries must be non-empty strings' }
        if ($t.Length -gt 4096) { throw 'targets entries exceed 4096 chars' }
    }
    switch ($op) {
        'file.read' {
            if ($Raw.PSObject.Properties['max_bytes']) {
                if (($Raw.max_bytes -isnot [int]) -and ($Raw.max_bytes -isnot [long]) -and ($Raw.max_bytes -isnot [int64])) {
                    throw 'max_bytes must be an integer'
                }
                if ([int64]$Raw.max_bytes -lt 1 -or [int64]$Raw.max_bytes -gt 1073741824) {
                    throw 'max_bytes must be in [1, 2**30]'
                }
            }
        }
        'file.write' {
            if ($targets.Count -ne 1) { throw 'file.write requires exactly one target' }
            if ($Raw.content -isnot [string]) { throw 'file.write requires string content' }
            if ($Raw.content.Length -gt 10485760) { throw 'content exceeds 10485760 chars' }
        }
        'file.move' {
            if ($targets.Count -ne 1) { throw 'file.move requires exactly one target' }
        }
        'file.copy' {
            if ($targets.Count -ne 1) { throw 'file.copy requires exactly one target' }
        }
        'file.delete' {
            if ($Raw.PSObject.Properties['dry_run'] -and $Raw.dry_run -isnot [bool]) {
                throw 'dry_run must be boolean'
            }
        }
        'dir.list' {
            if ($Raw.PSObject.Properties['depth']) {
                if (($Raw.depth -isnot [int]) -and ($Raw.depth -isnot [long]) -and ($Raw.depth -isnot [int64])) {
                    throw 'depth must be an integer'
                }
                if ([int64]$Raw.depth -lt 1 -or [int64]$Raw.depth -gt 32) { throw 'depth must be in [1, 32]' }
            }
            if ($Raw.PSObject.Properties['max_entries']) {
                if (($Raw.max_entries -isnot [int]) -and ($Raw.max_entries -isnot [long]) -and ($Raw.max_entries -isnot [int64])) {
                    throw 'max_entries must be an integer'
                }
                if ([int64]$Raw.max_entries -lt 1 -or [int64]$Raw.max_entries -gt 1000000) {
                    throw 'max_entries must be in [1, 1000000]'
                }
            }
        }
        'process.run' {
            if ($Raw.exe -isnot [string] -or [string]::IsNullOrWhiteSpace([string]$Raw.exe)) {
                throw 'process.run requires exe'
            }
            if (([string]$Raw.exe).Length -gt 2048) { throw 'exe exceeds 2048 chars' }
            if ($Raw.PSObject.Properties['args']) {
                if ($Raw.args -isnot [array]) { throw 'args must be an array' }
                if (@($Raw.args).Count -gt 512) { throw 'args exceeds 512 items' }
                foreach ($a in @($Raw.args)) {
                    if ($a -isnot [string]) { throw 'args must be strings' }
                    if ($a.Length -gt 4096) { throw 'args entries exceed 4096 chars' }
                }
            }
            if ($Raw.PSObject.Properties['cwd']) {
                if ($Raw.cwd -isnot [string] -or [string]::IsNullOrWhiteSpace([string]$Raw.cwd)) {
                    throw 'cwd must be a non-empty string'
                }
                if (([string]$Raw.cwd).Length -gt 4096) { throw 'cwd exceeds 4096 chars' }
            }
            if ($Raw.PSObject.Properties['timeout_ms']) {
                if (($Raw.timeout_ms -isnot [int]) -and ($Raw.timeout_ms -isnot [long]) -and ($Raw.timeout_ms -isnot [int64])) {
                    throw 'timeout_ms must be an integer'
                }
                if ([int64]$Raw.timeout_ms -lt 1000 -or [int64]$Raw.timeout_ms -gt 3600000) {
                    throw 'timeout_ms must be in [1000, 3600000]'
                }
            }
        }
    }
    if ($op -in @('file.move', 'file.copy') -and [string]::IsNullOrWhiteSpace([string]$Raw.dest)) {
        throw "$op requires dest"
    }
    if ($op -in @('file.move', 'file.copy')) {
        if ($Raw.dest -isnot [string]) { throw "$op requires dest string" }
        if (([string]$Raw.dest).Length -gt 4096) { throw 'dest exceeds 4096 chars' }
    }
    return $op
}

function Invoke-Op([pscustomobject]$Raw) {
    $op = Validate-Op $Raw
    switch ($op) {
        'file.read'   { return @{ Result = Read-File $Raw;    Policy = @{ decision = 'n/a' } ; Rejected = $false } }
        'file.write'  { return @{ Result = Write-File $Raw;   Policy = @{ decision = 'n/a' } ; Rejected = $false } }
        'file.move'   { return @{ Result = Move-Copy $Raw $false; Policy = @{ decision = 'n/a' }; Rejected = $false } }
        'file.copy'   { return @{ Result = Move-Copy $Raw $true;  Policy = @{ decision = 'n/a' }; Rejected = $false } }
        'dir.list'    { return @{ Result = Get-DirList $Raw;  Policy = @{ decision = 'n/a' } ; Rejected = $false } }
        'process.run' {
            if ($script:AllowRoots.Count -eq 0) {
                throw 'OPS-POLICY: process.run requires allow roots (OPS_ALLOW_ROOTS or -AllowRoot)'
            }
            $exePath = [string]$Raw.exe
            $resolvedExe = $exePath
            if (-not [IO.Path]::IsPathRooted($exePath)) {
                $cmd = Get-Command $exePath -ErrorAction SilentlyContinue | Select-Object -First 1
                if ($cmd -and $cmd.Source) { $resolvedExe = $cmd.Source }
            }
            if (-not (Test-Path -LiteralPath $resolvedExe)) { throw "executable not found: $exePath" }
            $realExe = Resolve-RealPath $resolvedExe
            $allowed = Test-InsideRoots $realExe
            if (-not $allowed) {
                $allow = Get-ProcessAllowlist
                $base = Split-Path -Leaf $realExe
                $stem = [IO.Path]::GetFileNameWithoutExtension($base)
                foreach ($entry in $allow) {
                    if ([IO.Path]::IsPathRooted($entry)) {
                        if ((Resolve-RealPath $entry) -eq $realExe) { $allowed = $true; break }
                    } elseif ($entry -eq $base -or $entry -eq $stem) {
                        $allowed = $true
                        break
                    }
                }
            }
            if (-not $allowed) { throw 'OPS-POLICY: process.run executable not authorized: ' + $realExe }
            $cwd = $null
            if ($Raw.PSObject.Properties['cwd'] -and $Raw.cwd) { $cwd = Resolve-Scoped ([string]$Raw.cwd) }
            $timeout = 120000
            if ($Raw.PSObject.Properties['timeout_ms'] -and $Raw.timeout_ms) { $timeout = [int]$Raw.timeout_ms }
            $argList = @()
            if ($Raw.PSObject.Properties['args'] -and $Raw.args) { $argList = @($Raw.args) }
            return @{ Result = Invoke-OpsProcess $resolvedExe $argList $cwd $timeout; Policy = @{ decision = 'n/a' }; Rejected = $false }
        }
        'env.info'    { return @{ Result = Get-EnvInfo;       Policy = @{ decision = 'n/a' } ; Rejected = $false } }
        'file.delete' {
            $resolved = Resolve-DeleteTargets @($Raw.targets)
            $policy = Get-OpsPolicy $resolved
            if ($policy.decision -eq 'reject') {
                return @{ Result = @{ plan = $resolved; decision = $policy }; Policy = $policy; Rejected = $true }
            }
            if ($Raw.PSObject.Properties['dry_run'] -and $Raw.dry_run) {
                return @{ Result = @{ plan = $resolved; decision = $policy; dry_run = $true; deleted = @() }; Policy = $policy; Rejected = $false }
            }
            $deleted = @()
            foreach ($r in $resolved) {
                Invoke-OpsDelete $r.absolute $policy.mode
                $deleted += $r.absolute
            }
            return @{ Result = @{ plan = $resolved; decision = $policy; deleted = $deleted }; Policy = $policy; Rejected = $false }
        }
    }
    throw "unknown op: $op"
}

$script:ExitCode = 1
$status = 'error'
$detail = $null
$result = $null
$policy = @{ decision = 'n/a'; mode = $null; reason = $null }
$resolvedAudit = @()
$op = ''
$raw = $null
$opsOutputEncoding = $null
$started = Get-Date
$requestId = 'REQ-' + (([guid]::NewGuid().ToString('N')).Substring(0, 12)).ToUpper()

try {
    $json = ''
    if ($OpFile -and $OpFile -ne '-') {
        $json = Get-Content -LiteralPath $OpFile -Raw -Encoding UTF8
    } else {
        $json = [Console]::In.ReadToEnd()
    }
    if ([string]::IsNullOrWhiteSpace($json)) { throw 'empty input' }
    $raw = $json | ConvertFrom-Json
    if ($raw.PSObject.Properties['id']) {
        $idVal = [string]$raw.id
        if ($idVal -match '^[A-Za-z0-9._-]{1,80}$') { $requestId = $idVal }
    }
    $op = Validate-Op $raw
    $invoked = Invoke-Op $raw
    $result = $invoked.Result
    $policy = $invoked.Policy
    if ($invoked.Rejected) {
        $status = 'rejected'
        $script:ExitCode = 2
    } else {
        $status = 'ok'
        $script:ExitCode = 0
    }
    if ($op -eq 'file.delete') {
        $resolvedAudit = @($result.plan | ForEach-Object {
            [pscustomobject]@{
                input      = $_.input
                absolute   = $_.absolute
                class      = $_.class
                evidence   = $_.evidence
                size_bytes = $_.size_bytes
                files      = $_.files
                dirs       = $_.dirs
            }
        })
    } else {
        foreach ($t in @($raw.targets)) {
            try {
                $resolvedAudit += [pscustomobject]@{ input = $t; absolute = Resolve-Scoped ([string]$t); class = 'n/a'; evidence = 'not a delete operation' }
            } catch {
                $resolvedAudit += [pscustomobject]@{ input = $t; absolute = $null; class = 'n/a'; evidence = 'resolution failed' }
            }
        }
    }
    if ($op -eq 'file.read' -and $result.files) {
        $encs = @($result.files | ForEach-Object { $_.encoding } | Where-Object { $_ } | Select-Object -Unique)
        if ($encs.Count -gt 0) { $opsOutputEncoding = $encs -join ',' }
    } elseif ($op -eq 'process.run' -and $result) {
        $encs = @($result.stdout_encoding, $result.stderr_encoding | Where-Object { $_ } | Select-Object -Unique)
        if ($encs.Count -gt 0) { $opsOutputEncoding = $encs -join ',' }
    }
    if ($status -eq 'ok' -and $result.dry_run) { $status = 'dry-run' }
} catch {
    if (-not $op) { $op = '?' }
    $detail = $_.Exception.Message
    if ($_.InvocationInfo -and $_.InvocationInfo.PositionMessage) {
        $detail = $detail + ' | ' + ($_.InvocationInfo.PositionMessage -replace "`r?`n", ' ').Trim()
    }
    if ($detail -like 'OPS-POLICY:*') {
        $status = 'rejected'
        $script:ExitCode = 2
    } else {
        $status = 'invalid'
        $script:ExitCode = 1
    }
}

$durationMs = [int]((Get-Date) - $started).TotalMilliseconds
$source = $env:OPS_SOURCE
if (-not $source) { $source = 'local' }
$auditTargets = @()
if ($raw -and $raw.targets -is [array]) {
    $auditTargets = @($raw.targets | Where-Object { $_ -is [string] })
}
$auditEntry = [pscustomobject]@{
    schema_version = '0.1.0-draft'
    audit_kind     = 'structured_operation'
    request_id     = $requestId
    timestamp      = (Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ss.fffZ')
    op             = $op
    targets        = $auditTargets
    resolved       = $resolvedAudit
    policy         = $policy
    result         = @{ status = $status; detail = $detail; exit_code = $script:ExitCode; duration_ms = $durationMs }
    env            = Get-EnvInfo
    source         = $source
    output_encoding = $opsOutputEncoding
}
try { Write-OpsAudit $auditEntry } catch {
    $detail = (($detail + '; ') -replace ';\s*$', '') + 'audit write failed: ' + $_.Exception.Message
    $status = 'error'
    $script:ExitCode = 1
}

$out = @{
    ok         = ($status -in @('ok', 'dry-run'))
    request_id = $requestId
    op         = $op
    status     = $status
    result     = $result
    audit      = $auditEntry.result
}
$out | ConvertTo-Json -Depth 10
exit $script:ExitCode
