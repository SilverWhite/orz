<#
    S4 wrapup evidence reader (host-side elevated): dumps the JSON files
    produced by the wrapup stage from C:\workspace\wrapup for diagnosis.
    ASCII only.
#>
[CmdletBinding()]
param(
    [string]$OutFile = 'D:\CLI\_windows_high_nist\vm-read-wrapup-result.txt'
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
        }
        catch {
            Start-Sleep -Seconds 10
        }
    }
    if (-not $up) {
        throw 'win-s4 not reachable within 240s'
    }
    $sess = New-PSSession -VMName 'win-s4' -Credential $cred
    $files = Invoke-Command -Session $sess -ScriptBlock {
        Write-Output '--- wrapup files ---'
        Get-ChildItem -LiteralPath 'C:\workspace\wrapup' -File -ErrorAction SilentlyContinue |
            Sort-Object Name | ForEach-Object { "$($_.Name)|$($_.Length)" }
        foreach ($name in @('wrapup-pre-clean.json', 'wrapup-apply.json', 'wrapup-summary.json')) {
            $p = Join-Path 'C:\workspace\wrapup' $name
            Write-Output "--- $name ---"
            if (Test-Path -LiteralPath $p) {
                Get-Content -LiteralPath $p -Raw
            }
            else {
                Write-Output 'MISSING'
            }
        }
        foreach ($name in @('cred-nonadmin.json', 'cred-highnist.json')) {
            $p = Join-Path 'C:\workspace\wrapup' $name
            Write-Output "--- $name ---"
            if (Test-Path -LiteralPath $p) {
                Get-Content -LiteralPath $p -Raw
            }
            else {
                Write-Output 'MISSING'
            }
        }
        foreach ($name in @('windows-native-run-observation-wrapup-1.json', 'windows-native-run-observation-wrapup-2.json')) {
            $p = Join-Path 'C:\workspace\wrapup' $name
            Write-Output "--- $name ---"
            if (Test-Path -LiteralPath $p) {
                try {
                    $o = Get-Content -LiteralPath $p -Raw | ConvertFrom-Json
                    Write-Output "outcome=$($o.outcome) exit=$($o.process.exit_code)"
                    if ($o.checks) {
                        $o.checks.PSObject.Properties | ForEach-Object { Write-Output "CHECK $($_.Name)=$($_.Value)" }
                    }
                    if ($o.diagnostics) {
                        $o.diagnostics | ForEach-Object { Write-Output "DIAG: $_" }
                    }
                    else {
                        Write-Output 'NO_DIAGS'
                    }
                }
                catch {
                    Write-Output "PARSE_ERR=$($_.Exception.Message)"
                }
            }
            else {
                Write-Output 'MISSING'
            }
        }
        foreach ($name in @('enforcement-probe-wrapup-1.json', 'enforcement-probe-wrapup-2.json')) {
            $p = Join-Path 'C:\workspace\wrapup' $name
            Write-Output "--- $name ---"
            if (Test-Path -LiteralPath $p) {
                Get-Content -LiteralPath $p -Raw | Select-Object -First 60
            }
            else {
                Write-Output 'MISSING'
            }
        }
    }
    foreach ($l in $files) { $lines.Add($l) }
    Remove-PSSession -Session $sess -ErrorAction SilentlyContinue
}
catch {
    $lines.Add("ERROR: $($_.Exception.Message)")
}
$lines | Set-Content -LiteralPath $OutFile -Encoding utf8
Write-Output "resultFile=$OutFile"
