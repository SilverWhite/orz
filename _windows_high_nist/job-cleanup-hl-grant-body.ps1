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
