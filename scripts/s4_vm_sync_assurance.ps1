<#
    S4 VM assurance package sync (host-side elevated): copies the full
    D:\CLI\assurance directory into the VM at C:\s4\assurance and verifies
    file counts + hashes.
    ASCII only.
#>
[CmdletBinding()]
param(
    [string]$OutFile = 'D:\CLI\_windows_high_nist\vm-sync-assurance-result.txt'
)

$ErrorActionPreference = 'Stop'
$lines = New-Object System.Collections.Generic.List[string]

try {
    $sec = ConvertTo-SecureString '123456' -AsPlainText -Force
    $cred = New-Object System.Management.Automation.PSCredential('HL', $sec)
    $sess = New-PSSession -VMName 'win-s4' -Credential $cred
    Invoke-Command -Session $sess -ScriptBlock {
        New-Item -ItemType Directory -Path 'C:\s4\assurance' -Force | Out-Null
    }
    $srcFiles = Get-ChildItem -LiteralPath 'D:\CLI\assurance' -File
    $copied = 0
    foreach ($f in $srcFiles) {
        Copy-Item -LiteralPath $f.FullName -Destination ('C:\s4\assurance\' + $f.Name) -ToSession $sess -Force
        $srcHash = (Get-FileHash -LiteralPath $f.FullName -Algorithm SHA256).Hash
        $dstHash = Invoke-Command -Session $sess -ArgumentList $f.Name -ScriptBlock {
            param($Name) (Get-FileHash -LiteralPath ('C:\s4\assurance\' + $Name) -Algorithm SHA256).Hash
        }
        if ($srcHash -eq $dstHash) { $copied++ } else { $lines.Add("HASH_MISMATCH $($f.Name)") }
    }
    $lines.Add("SYNCED=$copied/$($srcFiles.Count)")
    $guest = Invoke-Command -Session $sess -ScriptBlock {
        (Get-ChildItem -LiteralPath 'C:\s4\assurance' -File).Count
    }
    $lines.Add("GUEST_COUNT=$guest")
}
catch {
    $lines.Add("ERROR: $($_.Exception.Message)")
}
finally {
    if ($sess) { Remove-PSSession -Session $sess }
}
$lines | Set-Content -LiteralPath $OutFile -Encoding utf8
