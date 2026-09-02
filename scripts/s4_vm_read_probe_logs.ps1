<#
    S4: read high-nist probe debug logs from the VM workspace (host elevated).
    ASCII only.
#>
[CmdletBinding()]
param(
    [string]$OutFile = 'D:\CLI\_windows_high_nist\vm-read-probe-logs-result.txt'
)

$ErrorActionPreference = 'Stop'
$lines = New-Object System.Collections.Generic.List[string]
try {
    $sec = ConvertTo-SecureString '123456' -AsPlainText -Force
    $cred = New-Object System.Management.Automation.PSCredential('HL', $sec)
    $deadline = (Get-Date).AddSeconds(240)
    $up = $false
    while ((Get-Date) -lt $deadline) {
        try {
            $t = New-PSSession -VMName 'win-s4' -Credential $cred -ErrorAction Stop
            Remove-PSSession -Session $t -ErrorAction SilentlyContinue
            $up = $true
            break
        } catch {
            Start-Sleep -Seconds 10
        }
    }
    if (-not $up) {
        throw 'win-s4 not reachable within 240s'
    }
    $sess = New-PSSession -VMName 'win-s4' -Credential $cred
    $files = Invoke-Command -Session $sess -ScriptBlock {
        Write-Output '--- VM sandbox AC_TEMP code? ---'
        Select-String -LiteralPath 'C:\s4\assurance\windows_sandbox.py' -Pattern 'package temp setup','pkg_root','AC.Temp' -ErrorAction SilentlyContinue | ForEach-Object { "HIT: $($_.LineNumber) $($_.Line.Trim())" }
        Write-Output '--- obs diagnostics ---'
        $o = Get-Content -LiteralPath 'C:\workspace\high-nist\windows-native-run-observation-high-nist.json' -Raw | ConvertFrom-Json
        if ($o.diagnostics) { $o.diagnostics | ForEach-Object { "DIAG: $_" } } else { Write-Output 'NO_DIAGS' }
        Write-Output '--- Packages listing ---'
        Get-ChildItem -LiteralPath 'C:\Users\AgentUser\AppData\Local\Packages' -Force -ErrorAction SilentlyContinue | Select-Object Name | ForEach-Object { $_.Name }
        Write-Output '--- latest pkg AC Temp icacls ---'
        $latest = Get-ChildItem -LiteralPath 'C:\Users\AgentUser\AppData\Local\Packages' -Directory -Filter 'p2_native_run_*' -ErrorAction SilentlyContinue | Sort-Object LastWriteTime -Descending | Select-Object -First 1
        if ($latest) {
            & icacls.exe $latest.FullName 2>&1
            $act = Join-Path $latest.FullName 'AC\Temp'
            Write-Output "AC_TEMP=$act"
            if (Test-Path -LiteralPath $act) {
                & icacls.exe $act 2>&1
            } else {
                Write-Output 'AC_TEMP_MISSING'
            }
        }
        Write-Output '--- whoami-groups.txt ---'
        Get-Content -LiteralPath 'C:\workspace\high-nist\.whoami-groups.txt' -ErrorAction SilentlyContinue
        Write-Output '--- whoami-priv.txt ---'
        Get-Content -LiteralPath 'C:\workspace\high-nist\.whoami-priv.txt' -ErrorAction SilentlyContinue
        Write-Output '--- probe-env.txt ---'
        Get-Content -LiteralPath 'C:\workspace\high-nist\.probe-env.txt' -ErrorAction SilentlyContinue
        Write-Output '--- imports-ok.txt ---'
        Get-Content -LiteralPath 'C:\workspace\high-nist\imports-ok.txt' -ErrorAction SilentlyContinue
        Write-Output '--- imports-fail.txt ---'
        Get-Content -LiteralPath 'C:\workspace\high-nist\imports-fail.txt' -ErrorAction SilentlyContinue
        Write-Output '--- ICACLS workspace ---'
        & icacls.exe 'C:\workspace' 2>&1
        Write-Output '--- ICACLS workspace\high-nist ---'
        & icacls.exe 'C:\workspace\high-nist' 2>&1
        Write-Output '--- OBSERVATION DIAGNOSTICS ---'
        $o = Get-Content -LiteralPath 'C:\workspace\high-nist\windows-native-run-observation-high-nist.json' -Raw | ConvertFrom-Json
        if ($o.diagnostics) { $o.diagnostics | ForEach-Object { "DIAG: $_" } }
        Get-ChildItem -LiteralPath 'C:\workspace\high-nist' -Force -ErrorAction SilentlyContinue | Select-Object Name,Length | ForEach-Object { "$($_.Name)|$($_.Length)" }
        Get-Content -LiteralPath 'C:\workspace\high-nist\.enforcement-probe-high-nist.debug.log' -ErrorAction SilentlyContinue
        Get-Content -LiteralPath 'C:\workspace\high-nist\.enforcement-probe-crash.log' -ErrorAction SilentlyContinue
        Get-Content -LiteralPath 'C:\workspace\high-nist\enforcement-probe-high-nist.json' -ErrorAction SilentlyContinue
    }
    foreach ($l in $files) { $lines.Add($l) }
    Remove-PSSession -Session $sess -ErrorAction SilentlyContinue
} catch {
    $lines.Add("ERROR: $($_.Exception.Message)")
}
$lines | Set-Content -LiteralPath $OutFile -Encoding utf8
