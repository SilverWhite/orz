$ErrorActionPreference = 'Continue'
$status = 'C:\Users\1\harbor-proxy-status.txt'
$wslIp = '172.26.11.61'

function Log($msg) { Add-Content -LiteralPath $status -Value $msg -Encoding UTF8 }

Set-Content -LiteralPath $status -Value '=== Harbor portproxy setup ===' -Encoding UTF8

Log '--- delete old portproxy entries ---'
netsh interface portproxy delete v4tov4 listenport=80 listenaddress=0.0.0.0 2>&1 | ForEach-Object { Log $_ }
netsh interface portproxy delete v4tov4 listenport=443 listenaddress=0.0.0.0 2>&1 | ForEach-Object { Log $_ }

Log '--- add portproxy entries ---'
netsh interface portproxy add v4tov4 listenport=80 listenaddress=0.0.0.0 connectport=80 connectaddress=$wslIp 2>&1 | ForEach-Object { Log $_ }
netsh interface portproxy add v4tov4 listenport=443 listenaddress=0.0.0.0 connectport=443 connectaddress=$wslIp 2>&1 | ForEach-Object { Log $_ }

Log '--- firewall rules ---'
netsh advfirewall firewall delete rule name="Harbor-Http-80" 2>&1 | ForEach-Object { Log $_ }
netsh advfirewall firewall delete rule name="Harbor-Https-443" 2>&1 | ForEach-Object { Log $_ }
netsh advfirewall firewall add rule name="Harbor-Http-80" dir=in action=allow protocol=TCP localport=80 2>&1 | ForEach-Object { Log $_ }
netsh advfirewall firewall add rule name="Harbor-Https-443" dir=in action=allow protocol=TCP localport=443 2>&1 | ForEach-Object { Log $_ }

Log '--- portproxy list ---'
netsh interface portproxy show v4tov4 2>&1 | ForEach-Object { Log $_ }

Log '--- listeners on 80/443 ---'
netstat -ano | Select-String -Pattern ':(80|443)\s' | ForEach-Object { Log $_.Line.Trim() }

Log '=== done ==='
