$env:Path = [Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [Environment]::GetEnvironmentVariable('Path','User')
$ErrorActionPreference = 'Continue'
$outDir = 'C:\workspace\netcheck'
New-Item -ItemType Directory -Path $outDir -Force | Out-Null
$data = [ordered]@{
    ran_at = (Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ')
    processes = New-Object System.Collections.ArrayList
    config_dirs = New-Object System.Collections.ArrayList
    config_files = New-Object System.Collections.ArrayList
    ports = New-Object System.Collections.ArrayList
    listening = New-Object System.Collections.ArrayList
    curl = New-Object System.Collections.ArrayList
    direct_http = ''
    direct_exit = -1
}
$procs = @(Get-Process -ErrorAction SilentlyContinue | Where-Object { $_.ProcessName -match 'clash|mihomo|verge' })
foreach ($p in $procs) {
    [void]$data.processes.Add([ordered]@{ id = $p.Id; name = $p.ProcessName; session = $p.SessionId; path = $p.Path })
}
$proxyPids = @($data.processes | ForEach-Object { [int]$_.id })
$roots = @('C:\Users\HL\AppData\Roaming','C:\Users\HL\AppData\Local','C:\Users\HL\AppData\Local\Programs','C:\ProgramData','C:\Program Files','C:\Program Files (x86)')
foreach ($root in $roots) {
    if (-not (Test-Path -LiteralPath $root)) { continue }
    $dirs = @(Get-ChildItem -LiteralPath $root -Directory -ErrorAction SilentlyContinue | Where-Object { $_.Name -match 'clash|verge|mihomo' })
    foreach ($d in $dirs) {
        [void]$data.config_dirs.Add($d.FullName)
        foreach ($f in @(Get-ChildItem -LiteralPath $d.FullName -File -ErrorAction SilentlyContinue | Select-Object -First 25)) {
            [void]$data.config_files.Add("$($f.FullName)|$($f.Length)")
        }
        foreach ($s in @(Get-ChildItem -LiteralPath $d.FullName -Directory -ErrorAction SilentlyContinue | Select-Object -First 20)) {
            [void]$data.config_dirs.Add($s.FullName)
            foreach ($f in @(Get-ChildItem -LiteralPath $s.FullName -File -ErrorAction SilentlyContinue | Select-Object -First 25)) {
                [void]$data.config_files.Add("$($f.FullName)|$($f.Length)")
            }
        }
    }
}
$candidates = @($data.config_files | ForEach-Object { ($_ -split '\|')[0] } | Where-Object { $_ -match '\.ya?ml$' })
foreach ($c in $candidates) {
    try {
        $t = Get-Content -LiteralPath $c -Raw -ErrorAction Stop
        if ($t -match 'mixed-port:\s*["'']?(\d+)') { [void]$data.ports.Add([int]$Matches[1]) }
        if ($t -match '(?m)^port:\s*["'']?(\d+)') { [void]$data.ports.Add([int]$Matches[1]) }
        if ($t -match 'socks-port:\s*["'']?(\d+)') { [void]$data.ports.Add([int]$Matches[1]) }
    }
    catch { }
}
$listeningLines = @(netstat.exe -ano 2>$null | Select-String 'LISTENING')
foreach ($l in $listeningLines) {
    $parts = @($l.Line.Trim() -split '\s+')
    if ($parts.Count -ge 5) {
        $pidv = 0
        [void][int]::TryParse($parts[4], [ref]$pidv)
        if ($proxyPids -contains $pidv) {
            [void]$data.listening.Add([ordered]@{ proto = $parts[0]; addr = $parts[1]; state = $parts[3]; pid = $pidv })
            $addr = $parts[1]
            if ($addr -match ':(\d+)$') { [void]$data.ports.Add([int]$Matches[1]) }
        }
    }
}
$ports = @($data.ports | Select-Object -Unique | Select-Object -First 6)
foreach ($p in $ports) {
    foreach ($scheme in @('http', 'socks5')) {
        $proxy = if ($scheme -eq 'http') { "http://127.0.0.1:$p" } else { "socks5://127.0.0.1:$p" }
        $code = (& curl.exe -x $proxy -sS -o NUL -w '%{http_code}' --max-time 15 'https://api.deepseek.com' 2>$null)
        $ex = $LASTEXITCODE
        [void]$data.curl.Add([ordered]@{ scheme = $scheme; port = $p; http = "$code"; exit = $ex })
    }
}
$direct = (& curl.exe -sS -o NUL -w '%{http_code}' --max-time 15 'https://api.deepseek.com' 2>$null)
$data.direct_http = "$direct"
$data.direct_exit = $LASTEXITCODE
$jsonPath = Join-Path $outDir 'netcheck.json'
$data | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $jsonPath -Encoding utf8
Write-Output "NETCHECK_JSON=$jsonPath"
Write-Output "NETCHECK_PROC_COUNT=$($data.processes.Count)"
Write-Output "NETCHECK_PORTS=$($ports -join ',')"
Write-Output 'NETCHECK_DONE'
exit 0
