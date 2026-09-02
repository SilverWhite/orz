<#
    S4 VM hardening retry (host-side elevated):
      A) sync patched apply_hardening.ps1 into the VM
      B) cleanup: remove the diagnostic HL grant on C:\WINDOWS
      C) run apply_hardening.ps1 -Arm high-nist -EnableAppLocker
    ASCII only.
#>
[CmdletBinding()]
param(
    [string]$OutFile = 'D:\CLI\_windows_high_nist\vm-harden2-result.txt'
)

$ErrorActionPreference = 'Stop'
$lines = New-Object System.Collections.Generic.List[string]
$runner = 'D:\CLI\scripts\s4_vm_run_elev.ps1'

function Invoke-GuestJob {
    param([string]$Name, [string]$Body, [int]$TimeoutSeconds = 1200)
    $res = Join-Path 'D:\CLI\_windows_high_nist' ("job-$Name-result.txt")
    $bodyFile = Join-Path 'D:\CLI\_windows_high_nist' ("job-$Name-body.ps1")
    Set-Content -LiteralPath $bodyFile -Value $Body -Encoding utf8
    Remove-Item -LiteralPath $res -Force -ErrorAction SilentlyContinue
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $runner `
        -JobBodyFile $bodyFile -OutFile $res -TimeoutSeconds $TimeoutSeconds
    if ($LASTEXITCODE -ne 0) { throw "guest job $Name runner failed (exit $LASTEXITCODE)" }
    $lines.Add("===== JOB $Name =====")
    foreach ($l in (Get-Content -LiteralPath $res -Encoding UTF8)) { $lines.Add($l) }
}

try {
    $sec = ConvertTo-SecureString '123456' -AsPlainText -Force
    $cred = New-Object System.Management.Automation.PSCredential('HL', $sec)
    $sess = New-PSSession -VMName 'win-s4' -Credential $cred
    Copy-Item -LiteralPath 'D:\CLI\_windows_high_nist\hardening\apply_hardening.ps1' `
        -Destination 'C:\s4\_windows_high_nist\hardening\apply_hardening.ps1' -ToSession $sess -Force
    $srcHash = (Get-FileHash -LiteralPath 'D:\CLI\_windows_high_nist\hardening\apply_hardening.ps1' -Algorithm SHA256).Hash
    $dstHash = Invoke-Command -Session $sess -ScriptBlock {
        (Get-FileHash -LiteralPath 'C:\s4\_windows_high_nist\hardening\apply_hardening.ps1' -Algorithm SHA256).Hash
    }
    $lines.Add("SYNC_APPLY_HARDENING=$($srcHash -eq $dstHash)")
    Remove-PSSession -Session $sess

    $bodyB = @'
$acl = Get-Acl -LiteralPath 'C:\WINDOWS'
$removed = $false
$rules = @($acl.Access | Where-Object {
    $_.IdentityReference.Value -like '*\HL' -and
    $_.AccessControlType -eq [System.Security.AccessControl.AccessControlType]::Allow -and
    $_.FileSystemRights -eq [System.Security.AccessControl.FileSystemRights]::Modify
})
foreach ($r in $rules) {
    $null = $acl.RemoveAccessRule($r)
    $removed = $true
}
if ($removed) {
    Set-Acl -LiteralPath 'C:\WINDOWS' -AclObject $acl -ErrorAction Stop
    Write-Output "HL_GRANT_CLEANED=$removed"
} else {
    Write-Output 'HL_GRANT_NONE'
}
'@
    Invoke-GuestJob -Name 'cleanup-hl-grant' -Body $bodyB

    $bodyC = @"
`$env:Path = [Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [Environment]::GetEnvironmentVariable('Path','User')
New-Item -ItemType Directory -Path 'C:\s4\results' -Force | Out-Null
& C:\s4\_windows_high_nist\hardening\apply_hardening.ps1 -Arm high-nist -RunUser AgentUser -WorkspaceRoot C:\workspace -DeepSeekIp '221.204.163.76' -EnableAppLocker
exit `$LASTEXITCODE
"@
    Invoke-GuestJob -Name 'harden-high-nist' -Body $bodyC -TimeoutSeconds 1200
}
catch {
    $lines.Add("ERROR: $($_.Exception.Message)")
}
$lines | Set-Content -LiteralPath $OutFile -Encoding utf8
