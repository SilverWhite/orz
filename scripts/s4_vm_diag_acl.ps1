<#
    S4 VM ACL/token diagnostic (host-side elevated). ASCII only.
#>
[CmdletBinding()]
param(
    [string]$OutFile = 'D:\CLI\_windows_high_nist\vm-diag-acl-result.txt'
)

$ErrorActionPreference = 'Stop'
$lines = New-Object System.Collections.Generic.List[string]
$runner = 'D:\CLI\scripts\s4_vm_run_elev.ps1'

$body = @'
$ErrorActionPreference = 'Continue'
function Find-Rule {
    param($Acl, $User, $Type)
    foreach ($r in $Acl.Access) {
        if ($r.IdentityReference.Value -like "*$User" -and $r.AccessControlType -eq $Type) { return $r }
    }
    return $null
}
Write-Output '--- icacls control test on C:\s4\hardening ---'
icacls C:\s4\hardening /deny "AgentUser:(OI)(CI)(W,AD,DE,DC)" 2>&1 | ForEach-Object { Write-Output "CTL: $_" }
Write-Output "CTL_EXIT=$LASTEXITCODE"
icacls C:\s4\hardening /remove:d "AgentUser" 2>&1 | Out-Null

Write-Output '--- C:\WINDOWS ACL state: protected + leftover HL ACE? ---'
$wacl = Get-Acl -LiteralPath 'C:\WINDOWS'
Write-Output "PROTECTED=$($wacl.AreAccessRulesProtected)"
$hlRule = Find-Rule -Acl $wacl -User 'HL' -Type ([System.Security.AccessControl.AccessControlType]::Allow)
Write-Output "HL_MODIFY_ACE_PRESENT=$($null -ne $hlRule)"

Write-Output '--- .NET deny test on C:\ProgramData (add/verify/remove) ---'
try {
    $acl = Get-Acl -LiteralPath 'C:\ProgramData'
    $rule = [System.Security.AccessControl.FileSystemAccessRule]::new(
        'AgentUser',
        [System.Security.AccessControl.FileSystemRights]::WriteData -bor
            [System.Security.AccessControl.FileSystemRights]::CreateDirectories -bor
            [System.Security.AccessControl.FileSystemRights]::Delete -bor
            [System.Security.AccessControl.FileSystemRights]::DeleteSubdirectoriesAndFiles,
        [System.Security.AccessControl.InheritanceFlags]::ContainerInherit -bor [System.Security.AccessControl.InheritanceFlags]::ObjectInherit,
        [System.Security.AccessControl.PropagationFlags]::None,
        [System.Security.AccessControl.AccessControlType]::Deny)
    $acl.AddAccessRule($rule)
    Set-Acl -LiteralPath 'C:\ProgramData' -AclObject $acl -ErrorAction Stop
    $check = Get-Acl -LiteralPath 'C:\ProgramData'
    Write-Output "PD_DENY_STUCK=$($null -ne (Find-Rule -Acl $check -User 'AgentUser' -Type ([System.Security.AccessControl.AccessControlType]::Deny)))"
    $check.RemoveAccessRule($rule) | Out-Null
    Set-Acl -LiteralPath 'C:\ProgramData' -AclObject $check -ErrorAction Stop
    $check2 = Get-Acl -LiteralPath 'C:\ProgramData'
    Write-Output "PD_DENY_REMOVED=$($null -eq (Find-Rule -Acl $check2 -User 'AgentUser' -Type ([System.Security.AccessControl.AccessControlType]::Deny)))"
} catch { Write-Output "PD_ERR=$($_.Exception.Message)" }

Write-Output '--- .NET deny test on C:\WINDOWS (add/verify/remove) ---'
try {
    $acl = Get-Acl -LiteralPath 'C:\WINDOWS'
    $rule = [System.Security.AccessControl.FileSystemAccessRule]::new(
        'AgentUser',
        [System.Security.AccessControl.FileSystemRights]::WriteData -bor
            [System.Security.AccessControl.FileSystemRights]::CreateDirectories -bor
            [System.Security.AccessControl.FileSystemRights]::Delete -bor
            [System.Security.AccessControl.FileSystemRights]::DeleteSubdirectoriesAndFiles,
        [System.Security.AccessControl.InheritanceFlags]::ContainerInherit -bor [System.Security.AccessControl.InheritanceFlags]::ObjectInherit,
        [System.Security.AccessControl.PropagationFlags]::None,
        [System.Security.AccessControl.AccessControlType]::Deny)
    $acl.AddAccessRule($rule)
    Set-Acl -LiteralPath 'C:\WINDOWS' -AclObject $acl -ErrorAction Stop
    $check = Get-Acl -LiteralPath 'C:\WINDOWS'
    Write-Output "WIN_DENY_STUCK=$($null -ne (Find-Rule -Acl $check -User 'AgentUser' -Type ([System.Security.AccessControl.AccessControlType]::Deny)))"
    $check.RemoveAccessRule($rule) | Out-Null
    Set-Acl -LiteralPath 'C:\WINDOWS' -AclObject $check -ErrorAction Stop
    $check2 = Get-Acl -LiteralPath 'C:\WINDOWS'
    Write-Output "WIN_DENY_REMOVED=$($null -eq (Find-Rule -Acl $check2 -User 'AgentUser' -Type ([System.Security.AccessControl.AccessControlType]::Deny)))"
} catch { Write-Output "WIN_ERR=$($_.Exception.Message)" }
'@

$bodyFile = 'D:\CLI\_windows_high_nist\job-diag-acl-body.ps1'
Set-Content -LiteralPath $bodyFile -Value $body -Encoding utf8
try {
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $runner `
        -JobBodyFile $bodyFile -OutFile $OutFile -TimeoutSeconds 300
    if ($LASTEXITCODE -ne 0) { throw "diag job failed (exit $LASTEXITCODE)" }
}
catch {
    $lines.Add("ERROR: $($_.Exception.Message)")
    $lines | Set-Content -LiteralPath $OutFile -Encoding utf8
}
