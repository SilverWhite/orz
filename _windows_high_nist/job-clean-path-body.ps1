$p = [Environment]::GetEnvironmentVariable('Path','Machine')
$entries = $p -split ';' | ForEach-Object {
    $e = $_.TrimEnd('\')
    if ($e -and $e -ne 'C:\Program' -and $e -ne 'C:\Program\Scripts') { $_ }
}
$new = ($entries -join ';')
[Environment]::SetEnvironmentVariable('Path', $new, 'Machine')
Write-Output "PATH_FINAL=$([Environment]::GetEnvironmentVariable('Path','Machine'))"
