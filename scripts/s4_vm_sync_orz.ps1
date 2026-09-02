<#
    S4 sync freshly built Windows orz.exe into the VM (host-side elevated).

    Copies D:\CLI\orz\target\x86_64-pc-windows-msvc\release\orz.exe to
    C:\Program Files\orz\orz.exe, backing up the previous VM binary to
    D:\CLI\_windows_high_nist\backup\ first.  Hash-verified both ways.
    ASCII only.
#>
[CmdletBinding()]
param(
    [string]$OutFile = 'D:\CLI\_windows_high_nist\vm-sync-orz-result.txt'
)

$ErrorActionPreference = 'Stop'
$vmName = 'win-s4'
$localExe = 'D:\CLI\orz\target\release\orz.exe'
$remoteExe = 'C:\Program Files\orz\orz.exe'
$lines = New-Object System.Collections.Generic.List[string]

try {
    if (-not (Test-Path -LiteralPath $localExe)) {
        throw "local orz.exe missing: $localExe"
    }
    $localHash = (Get-FileHash -LiteralPath $localExe -Algorithm SHA256).Hash
    $localSize = (Get-Item -LiteralPath $localExe).Length
    $lines.Add("LOCAL_HASH=$localHash")
    $lines.Add("LOCAL_SIZE=$localSize")

    $sec = ConvertTo-SecureString '123456' -AsPlainText -Force
    $cred = New-Object System.Management.Automation.PSCredential('HL', $sec)
    $sess = $null
    $deadline = (Get-Date).AddSeconds(240)
    while (-not $sess -and (Get-Date) -lt $deadline) {
        try {
            $sess = New-PSSession -VMName $vmName -Credential $cred -ErrorAction Stop
        }
        catch {
            Start-Sleep -Seconds 10
        }
    }
    if (-not $sess) {
        throw 'win-s4 not reachable within 240s'
    }
    try {
        $remotePresent = Invoke-Command -Session $sess -ScriptBlock {
            param($p) (Test-Path -LiteralPath $p)
        } -ArgumentList $remoteExe
        $oldHash = ''
        if ($remotePresent) {
            $oldHash = Invoke-Command -Session $sess -ScriptBlock {
                param($p) (Get-FileHash -LiteralPath $p -Algorithm SHA256).Hash
            } -ArgumentList $remoteExe
            $lines.Add("REMOTE_OLD_HASH=$oldHash")
            if ($oldHash -eq $localHash) {
                $lines.Add('ORZ_ALREADY_CURRENT=True')
                $lines.Add('ORZ_SYNC_OK=True')
                $lines | Set-Content -LiteralPath $OutFile -Encoding utf8
                Write-Output "resultFile=$OutFile"
                exit 0
            }
            $backupDir = 'D:\CLI\_windows_high_nist\backup'
            New-Item -ItemType Directory -Path $backupDir -Force | Out-Null
            $backupPath = Join-Path $backupDir 'orz.exe.vm-pre-inject-2026-09-03.exe'
            Copy-Item -Path $remoteExe -Destination $backupPath -FromSession $sess -Force
            $bakHash = (Get-FileHash -LiteralPath $backupPath -Algorithm SHA256).Hash
            $lines.Add("BACKUP_HASH=$bakHash")
            if ($bakHash -ne $oldHash) {
                throw 'VM orz.exe backup hash mismatch'
            }
        }
        Copy-Item -LiteralPath $localExe -Destination $remoteExe -ToSession $sess -Force
        $newHash = Invoke-Command -Session $sess -ScriptBlock {
            param($p) (Get-FileHash -LiteralPath $p -Algorithm SHA256).Hash
        } -ArgumentList $remoteExe
        $lines.Add("REMOTE_NEW_HASH=$newHash")
        if ($newHash -ne $localHash) {
            throw 'VM orz.exe sync hash mismatch'
        }
        $lines.Add('ORZ_SYNC_OK=True')
    }
    finally {
        Remove-PSSession -Session $sess -ErrorAction SilentlyContinue
    }
}
catch {
    $lines.Add("ERROR: $($_.Exception.Message)")
    $lines.Add('ORZ_SYNC_OK=False')
}

$lines | Set-Content -LiteralPath $OutFile -Encoding utf8
Write-Output "resultFile=$OutFile"
if (@($lines | Where-Object { $_ -match '^ORZ_SYNC_OK=True$' }).Count -gt 0) {
    exit 0
}
exit 1
