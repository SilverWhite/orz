$env:Path = [Environment]::GetEnvironmentVariable('Path','Machine')
Write-Output "MACHINE_PATH=$env:Path"
foreach ($p in @('C:\Program','C:\Program Files\Python312')) {
    Write-Output "$p exists=$(Test-Path -LiteralPath $p)"
}
Write-Output '--- C:\Program contents ---'
Get-ChildItem -LiteralPath 'C:\Program' -ErrorAction SilentlyContinue | Select-Object -First 30 | ForEach-Object { Write-Output $_.Name }
