$ips = Resolve-DnsName api.deepseek.com -Type A -ErrorAction SilentlyContinue |
    Where-Object { $_.Type -eq 'A' } | Select-Object -ExpandProperty IPAddress -Unique
Write-Output "IPS=$($ips -join ',')"
