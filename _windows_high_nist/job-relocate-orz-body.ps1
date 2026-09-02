$src = 'C:\orz'
$dst = 'C:\Program Files\orz'
New-Item -ItemType Directory -Path $dst -Force | Out-Null
foreach ($f in @('orz.exe', 'orz-signer.exe', 'orz-acaf-provision.exe')) {
    $s = Join-Path $src $f
    $d = Join-Path $dst $f
    if (Test-Path -LiteralPath $d) { Remove-Item -LiteralPath $d -Force }
    if (-not (Test-Path -LiteralPath $s)) { Write-Output "MISSING $s"; continue }
    Move-Item -LiteralPath $s -Destination $d -Force
    Write-Output "MOVED $f size=$((Get-Item -LiteralPath $d).Length)"
}
