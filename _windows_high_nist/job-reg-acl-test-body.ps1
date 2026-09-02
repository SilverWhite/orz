$ErrorActionPreference = 'Continue'
Write-Output '--- volume info ---'
Get-Volume | Select-Object DriveLetter, FileSystem, DriveType, Size | ForEach-Object { Write-Output "VOL $($_.DriveLetter) fs=$($_.FileSystem) type=$($_.DriveType)" }
Write-Output '--- Get-Acl -LiteralPath HKLM:\SOFTWARE ---'
try {
    $acl = Get-Acl -LiteralPath 'HKLM:\SOFTWARE'
    Write-Output "GET_LITERAL_OK type=$($acl.GetType().Name) rules=$($acl.Access.Count)"
} catch { Write-Output "GET_LITERAL_ERR=$($_.Exception.Message)" }
Write-Output '--- Get-Acl -Path HKLM:\SOFTWARE ---'
try {
    $acl2 = Get-Acl -Path 'HKLM:\SOFTWARE'
    Write-Output "GET_PATH_OK type=$($acl2.GetType().Name) rules=$($acl2.Access.Count)"
} catch { Write-Output "GET_PATH_ERR=$($_.Exception.Message)" }
Write-Output '--- Set-Acl -LiteralPath HKLM:\SOFTWARE (add+remove readonly-ish deny for AgentUser) ---'
try {
    $rights = [System.Security.AccessControl.RegistryRights]::SetValue -bor `
        [System.Security.AccessControl.RegistryRights]::CreateSubKey -bor `
        [System.Security.AccessControl.RegistryRights]::Delete
    $rule = [System.Security.AccessControl.RegistryAccessRule]::new(
        'AgentUser', $rights, 'Deny', 'ContainerInherit,ObjectInherit', 'None')
    $acl.AddAccessRule($rule)
    Set-Acl -LiteralPath 'HKLM:\SOFTWARE' -AclObject $acl -ErrorAction Stop
    Write-Output 'SET_LITERAL_OK'
    $acl.RemoveAccessRule($rule) | Out-Null
    Set-Acl -LiteralPath 'HKLM:\SOFTWARE' -AclObject $acl -ErrorAction Stop
    Write-Output 'SET_LITERAL_REMOVE_OK'
} catch { Write-Output "SET_LITERAL_ERR=$($_.Exception.Message)" }
Write-Output '--- Set-Acl -Path HKLM:\SOFTWARE (add+remove) ---'
try {
    $acl3 = Get-Acl -LiteralPath 'HKLM:\SOFTWARE'
    $rule3 = [System.Security.AccessControl.RegistryAccessRule]::new(
        'AgentUser', $rights, 'Deny', 'ContainerInherit,ObjectInherit', 'None')
    $acl3.AddAccessRule($rule3)
    Set-Acl -Path 'HKLM:\SOFTWARE' -AclObject $acl3 -ErrorAction Stop
    Write-Output 'SET_PATH_OK'
    $acl3.RemoveAccessRule($rule3) | Out-Null
    Set-Acl -Path 'HKLM:\SOFTWARE' -AclObject $acl3 -ErrorAction Stop
    Write-Output 'SET_PATH_REMOVE_OK'
} catch { Write-Output "SET_PATH_ERR=$($_.Exception.Message)" }
