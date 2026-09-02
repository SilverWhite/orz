<#
    S4 copy helper (run elevated): copies orz tools + hardening/probe/runner
    files into the win-s4 VM via PowerShell Direct, verifies SHA256, and
    reports guest Python availability.
    ASCII only.
#>
[CmdletBinding()]
param(
    [string]$OutFile = 'D:\CLI\_windows_high_nist\vm-copy-result.txt'
)

$ErrorActionPreference = 'Stop'
$lines = New-Object System.Collections.Generic.List[string]

try {
    $sec = ConvertTo-SecureString '123456' -AsPlainText -Force
    $cred = New-Object System.Management.Automation.PSCredential('HL', $sec)
    $sess = New-PSSession -VMName 'win-s4' -Credential $cred

    $targets = @(
        @{ Src = 'D:\tb-eval\orz-windows\orz.exe';                    Dst = 'C:\orz\orz.exe' },
        @{ Src = 'D:\tb-eval\orz-windows\orz-signer.exe';             Dst = 'C:\orz\orz-signer.exe' },
        @{ Src = 'D:\tb-eval\orz-windows\orz-acaf-provision.exe';     Dst = 'C:\orz\orz-acaf-provision.exe' },
        @{ Src = 'D:\CLI\_windows_high_nist\hardening\apply_hardening.ps1';  Dst = 'C:\s4\hardening\apply_hardening.ps1' },
        @{ Src = 'D:\CLI\_windows_high_nist\policy\enforcement_probe.ps1';  Dst = 'C:\s4\policy\enforcement_probe.ps1' },
        @{ Src = 'D:\CLI\_windows_high_nist\run\run_enforcement_probe.ps1'; Dst = 'C:\s4\run\run_enforcement_probe.ps1' },
        @{ Src = 'D:\CLI\scripts\run_windows_native_sandbox_command.py';     Dst = 'C:\s4\scripts\run_windows_native_sandbox_command.py' },
        @{ Src = 'D:\CLI\scripts\run_windows_native_sandbox_probe.py';       Dst = 'C:\s4\scripts\run_windows_native_sandbox_probe.py' },
        @{ Src = 'D:\CLI\scripts\orz_acaf_run.ps1';                          Dst = 'C:\s4\scripts\orz_acaf_run.ps1' },
        @{ Src = 'D:\CLI\assurance\windows_sandbox.py';                      Dst = 'C:\s4\assurance\windows_sandbox.py' },
        @{ Src = 'D:\CLI\assurance\sandbox_verifier.py';                     Dst = 'C:\s4\assurance\sandbox_verifier.py' },
        @{ Src = 'D:\CLI\assurance\windows-native-sandbox-run-v0.1.schema.json';         Dst = 'C:\s4\assurance\windows-native-sandbox-run-v0.1.schema.json' },
        @{ Src = 'D:\CLI\assurance\windows-native-sandbox-observation-v0.1.schema.json'; Dst = 'C:\s4\assurance\windows-native-sandbox-observation-v0.1.schema.json' },
        @{ Src = 'D:\CLI\assurance\windows-native-sandbox-profile-v0.1.json';             Dst = 'C:\s4\assurance\windows-native-sandbox-profile-v0.1.json' },
        @{ Src = 'D:\CLI\assurance\windows-native-sandbox-profile-v0.1.schema.json';     Dst = 'C:\s4\assurance\windows-native-sandbox-profile-v0.1.schema.json' }
    )

    Invoke-Command -Session $sess -ScriptBlock {
        foreach ($d in @('C:\orz', 'C:\s4\hardening', 'C:\s4\policy', 'C:\s4\run', 'C:\s4\scripts', 'C:\s4\assurance', 'C:\workspace')) {
            New-Item -ItemType Directory -Path $d -Force | Out-Null
        }
    }

    $allOk = $true
    foreach ($t in $targets) {
        Copy-Item -LiteralPath $t.Src -Destination $t.Dst -ToSession $sess -Force
        $srcHash = (Get-FileHash -LiteralPath $t.Src -Algorithm SHA256).Hash
        $dstHash = Invoke-Command -Session $sess -ScriptBlock {
            param($p) (Get-FileHash -LiteralPath $p -Algorithm SHA256).Hash
        } -ArgumentList $t.Dst
        $ok = ($srcHash -eq $dstHash)
        if (-not $ok) { $allOk = $false }
        $lines.Add("COPY $($t.Dst) ok=$ok")
    }
    $lines.Add("ALL_COPY_OK=$allOk")

    $guest = Invoke-Command -Session $sess -ScriptBlock {
        $py = Get-Command python -ErrorAction SilentlyContinue
        $py3 = Get-Command python3 -ErrorAction SilentlyContinue
        $id = [System.Security.Principal.WindowsIdentity]::GetCurrent()
        [pscustomobject]@{
            Python = if ($py) { $py.Source } else { 'MISSING' }
            Python3 = if ($py3) { $py3.Source } else { 'MISSING' }
            FreeC = [math]::Round((Get-PSDrive C).Free / 1GB, 1)
            FreeD = [math]::Round((Get-PSDrive D).Free / 1GB, 1)
            User = $id.Name
        } | Format-List | Out-String
    }
    $lines.Add(($guest | Out-String).Trim())
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
