Write-Output '--- whoami /groups (admin + integrity) ---'
whoami /groups | Select-String -Pattern 'S-1-5-32-544|Mandatory' | ForEach-Object { Write-Output $_.Line }
Write-Output '--- whoami /priv ---'
whoami /priv
Write-Output "PRIV_EXIT=$LASTEXITCODE"
Write-Output '--- net session ---'
net session 2>&1 | Select-Object -First 3 | ForEach-Object { Write-Output "NET: $_" }
Write-Output "NET_EXIT=$LASTEXITCODE"
$id = [System.Security.Principal.WindowsIdentity]::GetCurrent()
$p = [System.Security.Principal.WindowsPrincipal]::new($id)
Write-Output "ISADMIN=$($p.IsInRole([System.Security.Principal.WindowsBuiltInRole]::Administrator))"
