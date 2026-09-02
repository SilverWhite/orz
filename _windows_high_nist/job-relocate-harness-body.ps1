$base = 'C:\s4\_windows_high_nist'
New-Item -ItemType Directory -Path (Join-Path $base 'run') -Force | Out-Null
New-Item -ItemType Directory -Path (Join-Path $base 'policy') -Force | Out-Null
New-Item -ItemType Directory -Path (Join-Path $base 'hardening') -Force | Out-Null
foreach ($d in @('run','policy','hardening')) {
    $src = Join-Path 'C:\s4' $d
    foreach ($f in (Get-ChildItem -LiteralPath $src -File)) {
        $dst = Join-Path (Join-Path $base $d) $f.Name
        Move-Item -LiteralPath $f.FullName -Destination $dst -Force
        Write-Output "MOVED $($f.Name) -> $dst"
    }
}
