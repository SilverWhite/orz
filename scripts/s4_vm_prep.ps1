<#
    S4 VM prep helper (run elevated): checks python usability, provisions
    ACAF keystore+manifest in the VM, persists ORZ_ACAF_* machine env vars,
    and reports the PowerShell Direct token integrity.
    ASCII only.
#>
[CmdletBinding()]
param(
    [string]$OutFile = 'D:\CLI\_windows_high_nist\vm-prep-result.txt'
)

$ErrorActionPreference = 'Stop'
$lines = New-Object System.Collections.Generic.List[string]

try {
    $sec = ConvertTo-SecureString '123456' -AsPlainText -Force
    $cred = New-Object System.Management.Automation.PSCredential('HL', $sec)
    $sess = New-PSSession -VMName 'win-s4' -Credential $cred

    $guest = Invoke-Command -Session $sess -ScriptBlock {
        $out = New-Object System.Collections.Generic.List[string]
        $id = [System.Security.Principal.WindowsIdentity]::GetCurrent()
        $highIL = $false
        foreach ($g in $id.Groups) {
            if ($g.Value -eq 'S-1-16-12288') { $highIL = $true }
        }
        $out.Add("TOKEN user=$($id.Name) highIL=$highIL admin=" + [System.Security.Principal.WindowsPrincipal]::new($id).IsInRole([System.Security.Principal.WindowsBuiltInRole]::Administrator))

        $pyOut = $null
        $pyCode = 9009
        try {
            $pyOut = & python -c "import sys; print(sys.version)" 2>&1
            $pyCode = $LASTEXITCODE
        } catch {
            $pyOut = $_.Exception.Message
        }
        $out.Add("PYTHON exit=$pyCode out=$($pyOut -join ' ')".Trim())

        $pyLauncher = $null
        $pyLauncherCode = 9009
        try {
            $pyLauncher = & py -3 -c "import sys; print(sys.version)" 2>&1
            $pyLauncherCode = $LASTEXITCODE
        } catch {
            $pyLauncher = $_.Exception.Message
        }
        $out.Add("PY3 exit=$pyLauncherCode out=$($pyLauncher -join ' ')".Trim())

        $provisionOut = & C:\orz\orz-acaf-provision.exe C:\Users\HL\AppData\Local\orz\acaf C:\Users\HL\AppData\Local\orz\acaf\signer-manifest.json 2>&1
        $provisionCode = $LASTEXITCODE
        $out.Add("PROVISION exit=$provisionCode")
        foreach ($l in $provisionOut) { $out.Add("PROVISION: $l") }

        $manifest = 'C:\Users\HL\AppData\Local\orz\acaf\signer-manifest.json'
        $keystore = 'C:\Users\HL\AppData\Local\orz\acaf'
        [Environment]::SetEnvironmentVariable('ORZ_ACAF_KEYSTORE', $keystore, 'Machine')
        [Environment]::SetEnvironmentVariable('ORZ_ACAF_MANIFEST', $manifest, 'Machine')
        [Environment]::SetEnvironmentVariable('ORZ_ACAF_BINARY', 'C:\orz\orz-signer.exe', 'Machine')
        [Environment]::SetEnvironmentVariable('ORZ_ACAF_FAIL_CLOSED', '1', 'Machine')
        $out.Add("ENV machine ORZ_ACAF_KEYSTORE=$([Environment]::GetEnvironmentVariable('ORZ_ACAF_KEYSTORE','Machine'))")
        $out.Add("ENV machine ORZ_ACAF_MANIFEST=$([Environment]::GetEnvironmentVariable('ORZ_ACAF_MANIFEST','Machine'))")
        $out.Add("ENV machine ORZ_ACAF_BINARY=$([Environment]::GetEnvironmentVariable('ORZ_ACAF_BINARY','Machine'))")
        $out.Add("ENV machine ORZ_ACAF_FAIL_CLOSED=$([Environment]::GetEnvironmentVariable('ORZ_ACAF_FAIL_CLOSED','Machine'))")

        if (Test-Path -LiteralPath $manifest) {
            $m = Get-Content -LiteralPath $manifest -Raw | ConvertFrom-Json
            $out.Add("MANIFEST manifest_version=$($m.manifest_version) signer_revision=$($m.signer_revision) binary_name=$($m.binary_name) binary_sha256=$($m.binary_sha256)")
        } else {
            $out.Add("MANIFEST missing: $manifest")
        }
        $out
    }
    foreach ($l in $guest) { $lines.Add($l) }
}
catch {
    $lines.Add("ERROR: $($_.Exception.Message)")
    $lines.Add("STACK: $($_.ScriptStackTrace)")
}
finally {
    if ($sess) {
        Remove-PSSession -Session $sess
    }
}
$lines | Set-Content -LiteralPath $OutFile -Encoding utf8
