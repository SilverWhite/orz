<#
    apply_hardening.ps1 — Windows HIGH-NIST 加固脚本（VM 模板级，P0-0l-②）

    把 WINDOWS_HIGH_NIST_MAX_FRICTION_DESIGN §3 的 N×F×P 格映射为机器/账户级
    策略。spawn 期的 per-token 构造（受限 token / LOW IL / AppContainer /
    TEMP 重定向）由 assurance/windows_sandbox.py 运行环境负责，本脚本只做
    模板级：ACL、注册表、账户、防火墙默认出站策略 + 每任务 allowlist、
    AppLocker 白名单。

    用法（在硬化 VM 模板上，管理员 PowerShell）：
      .\apply_hardening.ps1 -Arm high-nist -RunUser AgentUser -WorkspaceRoot D:\workspace
      .\apply_hardening.ps1 -Arm high-nist -Revert          # 撤销本脚本做的变更

    三臂：
      control    — 不做任何策略（仅验证脚本可跑，输出基线）。
      non-admin  — Privilege·non-admin：UAC 收紧、RunUser 标准账户、
                   系统目录/盘符根 deny-write、HKLM 只读、禁提权路径。
      high-nist  — non-admin + Filesystem·readonly-os/freeze-home（profile
                   冻结 + HKCU 冻结 + TEMP 重定向目录）+ Network·
                   restrict-egress（出站默认 block + allowlist）+ AppLocker。

    幂等：重复运行不重复叠加；-Revert 删除本脚本添加的 deny ACE/规则/策略。
#>

[CmdletBinding(SupportsShouldProcess = $true)]
param(
    [Parameter(Mandatory = $true)]
    [ValidateSet('control', 'non-admin', 'high-nist')]
    [string]$Arm,

    [string]$RunUser = 'AgentUser',

    [string]$WorkspaceRoot = 'D:\workspace',

    [string[]]$AllowlistIps = @(),

    [string]$DeepSeekIp = '',

    [switch]$EnableAppLocker,

    [switch]$DisableBreakGlassAdmin,

    [switch]$Revert,

    [string]$LogPath = ''
)

$ErrorActionPreference = 'Stop'
$script:StepFailures = 0
$RulePrefix = 'GSA-HIGH-NIST'

if (-not $LogPath) {
    $LogPath = Join-Path $PSScriptRoot "logs\hardening-$Arm-$([DateTime]::Now.ToString('yyyyMMdd-HHmmss')).log"
}
$LogDir = Split-Path -Parent $LogPath
if ($LogDir -and -not (Test-Path -LiteralPath $LogDir)) {
    New-Item -ItemType Directory -Path $LogDir -Force | Out-Null
}

function Write-Step {
    param([string]$Message)
    $line = "[$(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')] $Message"
    Write-Host $line
    Add-Content -LiteralPath $LogPath -Value $line -Encoding utf8
}

function Assert-Admin {
    $identity = [System.Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = [System.Security.Principal.WindowsPrincipal]::new($identity)
    if (-not $principal.IsInRole([System.Security.Principal.WindowsBuiltInRole]::Administrator)) {
        throw "apply_hardening.ps1 需要管理员权限（在硬化 VM 模板上以管理员运行）。"
    }
}

function Get-AclRuleIdentitySid {
    param([string]$Identity)
    try {
        $acct = [System.Security.Principal.NTAccount]::new($Identity)
        $sid = $acct.Translate([System.Security.Principal.SecurityIdentifier])
        return $sid.Value
    }
    catch {
        return $Identity
    }
}

function Get-AclInheritance {
    param([string]$Inheritance)
    if ($Inheritance -eq '(OI)(CI)') {
        return [System.Security.AccessControl.InheritanceFlags]::ContainerInherit -bor `
            [System.Security.AccessControl.InheritanceFlags]::ObjectInherit
    }
    return [System.Security.AccessControl.InheritanceFlags]::None
}

function Invoke-AclDeny {
    param(
        [string]$Path,
        [string]$User,
        [string]$Inheritance = '(OI)(CI)'
    )
    if ($PSCmdlet.ShouldProcess($Path, 'deny-write ACL')) {
        try {
            $acl = Get-Acl -LiteralPath $Path -ErrorAction Stop
            # W=写文件, AD=建子目录, DE=删文件, DC=删子目录——deny-write 语义
            # 必须同时覆盖文件写与目录创建（仅 W 会漏掉 mkdir）。
            $rights = [System.Security.AccessControl.FileSystemRights]::WriteData -bor `
                [System.Security.AccessControl.FileSystemRights]::CreateDirectories -bor `
                [System.Security.AccessControl.FileSystemRights]::Delete -bor `
                [System.Security.AccessControl.FileSystemRights]::DeleteSubdirectoriesAndFiles
            $rule = [System.Security.AccessControl.FileSystemAccessRule]::new(
                $User,
                $rights,
                (Get-AclInheritance $Inheritance),
                [System.Security.AccessControl.PropagationFlags]::None,
                [System.Security.AccessControl.AccessControlType]::Deny
            )
            $acl.AddAccessRule($rule)
            Set-Acl -LiteralPath $Path -AclObject $acl -ErrorAction Stop
            Write-Step "OK   acl deny $Path ($Inheritance)"
        }
        catch {
            Write-Step "FAIL acl deny $Path : $($_.Exception.Message)"
            $script:StepFailures++
        }
    }
}

function Invoke-AclDenyRemove {
    param(
        [string]$Path,
        [string]$User,
        [string]$Inheritance = '(OI)(CI)'
    )
    if ($PSCmdlet.ShouldProcess($Path, 'remove deny-write ACE')) {
        try {
            $acl = Get-Acl -LiteralPath $Path -ErrorAction Stop
            $targetSid = Get-AclRuleIdentitySid $User
            $rules = @($acl.Access | Where-Object {
                $_.AccessControlType -eq [System.Security.AccessControl.AccessControlType]::Deny -and
                (Get-AclRuleIdentitySid $_.IdentityReference.Value) -eq $targetSid
            })
            foreach ($r in $rules) {
                $null = $acl.RemoveAccessRule($r)
            }
            if ($rules.Count -gt 0) {
                Set-Acl -LiteralPath $Path -AclObject $acl -ErrorAction Stop
            }
            Write-Step "OK   acl deny removed $Path"
        }
        catch {
            Write-Step "FAIL acl deny remove $Path : $($_.Exception.Message)"
            $script:StepFailures++
        }
    }
}

function Invoke-AclGrant {
    param(
        [string]$Path,
        [string]$User,
        [string]$Inheritance = '(OI)(CI)',
        [string]$Rights = 'Modify'
    )
    if ($PSCmdlet.ShouldProcess($Path, 'grant ACL')) {
        try {
            $acl = Get-Acl -LiteralPath $Path -ErrorAction Stop
            $rightsEnum = [System.Security.AccessControl.FileSystemRights]::$Rights
            $rule = [System.Security.AccessControl.FileSystemAccessRule]::new(
                $User,
                $rightsEnum,
                (Get-AclInheritance $Inheritance),
                [System.Security.AccessControl.PropagationFlags]::None,
                [System.Security.AccessControl.AccessControlType]::Allow
            )
            $acl.AddAccessRule($rule)
            Set-Acl -LiteralPath $Path -AclObject $acl -ErrorAction Stop
            Write-Step "OK   acl grant $Path ($Rights)"
        }
        catch {
            Write-Step "FAIL acl grant $Path : $($_.Exception.Message)"
            $script:StepFailures++
        }
    }
}

function Invoke-AclGrantRemove {
    param(
        [string]$Path,
        [string]$User,
        [string]$Inheritance = '(OI)(CI)',
        [string]$Rights = 'Modify'
    )
    if ($PSCmdlet.ShouldProcess($Path, 'remove grant ACE')) {
        try {
            $acl = Get-Acl -LiteralPath $Path -ErrorAction Stop
            $targetSid = Get-AclRuleIdentitySid $User
            $rules = @($acl.Access | Where-Object {
                $_.AccessControlType -eq [System.Security.AccessControl.AccessControlType]::Allow -and
                (Get-AclRuleIdentitySid $_.IdentityReference.Value) -eq $targetSid
            })
            foreach ($r in $rules) {
                $null = $acl.RemoveAccessRule($r)
            }
            if ($rules.Count -gt 0) {
                Set-Acl -LiteralPath $Path -AclObject $acl -ErrorAction Stop
            }
            Write-Step "OK   acl grant removed $Path"
        }
        catch {
            Write-Step "FAIL acl grant remove $Path : $($_.Exception.Message)"
            $script:StepFailures++
        }
    }
}

function Invoke-RegistryDeny {
    param(
        [string]$KeyPath,
        [string]$User
    )
    if (-not $PSCmdlet.ShouldProcess($KeyPath, 'deny-write registry ACE')) {
        return
    }
    try {
        $acl = Get-Acl -Path $KeyPath -ErrorAction Stop
        $rights = [System.Security.AccessControl.RegistryRights]::SetValue -bor `
            [System.Security.AccessControl.RegistryRights]::CreateSubKey -bor `
            [System.Security.AccessControl.RegistryRights]::Delete
        $rule = [System.Security.AccessControl.RegistryAccessRule]::new(
            $User,
            $rights,
            [System.Security.AccessControl.InheritanceFlags]::ContainerInherit -bor `
                [System.Security.AccessControl.InheritanceFlags]::ObjectInherit,
            [System.Security.AccessControl.PropagationFlags]::None,
            [System.Security.AccessControl.AccessControlType]::Deny
        )
        $acl.AddAccessRule($rule)
        Set-Acl -Path $KeyPath -AclObject $acl -ErrorAction Stop
        Write-Step "OK   registry deny $KeyPath"
    }
    catch {
        Write-Step "FAIL registry deny $KeyPath : $($_.Exception.Message)"
        $script:StepFailures++
    }
}

function Invoke-RegistryDenyRemove {
    param(
        [string]$KeyPath,
        [string]$User
    )
    if (-not $PSCmdlet.ShouldProcess($KeyPath, 'remove deny-write ACE')) {
        return
    }
    try {
        $acl = Get-Acl -Path $KeyPath -ErrorAction Stop
        $rights = [System.Security.AccessControl.RegistryRights]::SetValue -bor `
            [System.Security.AccessControl.RegistryRights]::CreateSubKey -bor `
            [System.Security.AccessControl.RegistryRights]::Delete
        $rule = [System.Security.AccessControl.RegistryAccessRule]::new(
            $User,
            $rights,
            [System.Security.AccessControl.InheritanceFlags]::ContainerInherit -bor `
                [System.Security.AccessControl.InheritanceFlags]::ObjectInherit,
            [System.Security.AccessControl.PropagationFlags]::None,
            [System.Security.AccessControl.AccessControlType]::Deny
        )
        $acl.RemoveAccessRule($rule) | Out-Null
        Set-Acl -Path $KeyPath -AclObject $acl -ErrorAction Stop
        Write-Step "OK   registry deny removed $KeyPath"
    }
    catch {
        Write-Step "FAIL registry deny remove $KeyPath : $($_.Exception.Message)"
        $script:StepFailures++
    }
}

function Set-RegistryDword {
    param(
        [string]$Key,
        [string]$Name,
        [int]$Value
    )
    if (-not $PSCmdlet.ShouldProcess("$Key\$Name", 'set registry dword')) {
        return
    }
    try {
        if (-not (Test-Path -LiteralPath $Key)) {
            New-Item -Path $Key -Force | Out-Null
        }
        Set-ItemProperty -LiteralPath $Key -Name $Name -Value $Value -Type DWord
        Write-Step "OK   reg $Key\$Name = $Value"
    }
    catch {
        Write-Step "FAIL reg $Key\$Name : $($_.Exception.Message)"
        $script:StepFailures++
    }
}

function Ensure-RunUser {
    if ($PSCmdlet.ShouldProcess($RunUser, 'ensure standard local user')) {
        try {
            if (-not (Get-LocalUser -Name $RunUser -ErrorAction SilentlyContinue)) {
                $password = [System.Security.SecureString]::new()
                New-LocalUser -Name $RunUser -Password $password -PasswordNeverExpires `
                    -Description 'HIGH-NIST run user' | Out-Null
                Write-Step "OK   created local user $RunUser"
            }
            else {
                Write-Step "OK   local user $RunUser exists"
            }
            $adminGroup = Get-LocalGroup -Name 'Administrators'
            if (Get-LocalGroupMember -Group $adminGroup -ErrorAction SilentlyContinue |
                Where-Object { $_.Name -like "*$RunUser" }) {
                Remove-LocalGroupMember -Group $adminGroup -Member $RunUser
                Write-Step "OK   removed $RunUser from Administrators"
            }
            if (-not (Get-LocalGroupMember -Group 'Users' -ErrorAction SilentlyContinue |
                Where-Object { $_.Name -like "*$RunUser" })) {
                Add-LocalGroupMember -Group 'Users' -Member $RunUser
                Write-Step "OK   added $RunUser to Users"
            }
        }
        catch {
            Write-Step "FAIL ensure user $RunUser : $($_.Exception.Message)"
            $script:StepFailures++
        }
    }
}

function Get-RunUserProfileRoot {
    param([string]$Name)
    # 固定为 %SystemDrive%\Users\<RunUser>：脚本可能以 SYSTEM 运行
    # （SYSTEM 的 USERPROFILE 是 C:\WINDOWS\system32\config\systemprofile，
    # 用它推导会把 RunUser profile 建到错误位置——S4 实机发现）。
    return Join-Path (Join-Path $env:SystemDrive 'Users') $Name
}

function Ensure-RunUserProfileDirectories {
    param([string]$ProfileRoot)
    foreach ($dir in @(
        $ProfileRoot,
        (Join-Path $ProfileRoot 'AppData\Roaming'),
        (Join-Path $ProfileRoot 'AppData\Local')
    )) {
        if (-not (Test-Path -LiteralPath $dir)) {
            New-Item -ItemType Directory -Path $dir -Force | Out-Null
        }
    }
}

function Wait-RunUserHiveSettled {
    param([string]$ProfileRoot)
    # S4 5023 candidate fix: apply's HKCU-freeze reg load/unload cycle can
    # leave the NTUSER.DAT hive in a transient state; a LoadUserProfileW
    # started immediately afterwards fails with WinError 5023 ("group or
    # resource is not in the correct state").  Poll by mounting the same
    # file under a scratch HKU key until the load succeeds again (and is
    # then unloaded), proving the hive is ready for the profile service.
    $hiveFile = Join-Path $ProfileRoot 'NTUSER.DAT'
    if (-not (Test-Path -LiteralPath $hiveFile)) {
        return
    }
    $deadline = (Get-Date).AddSeconds(45)
    $settled = $false
    $prevEap = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    while ((Get-Date) -lt $deadline) {
        $mount = "gsa_hive_ready_$([guid]::NewGuid().ToString('N'))"
        & reg.exe load "HKU\$mount" $hiveFile 2>&1 | Out-Null
        if ($LASTEXITCODE -eq 0) {
            & reg.exe unload "HKU\$mount" 2>&1 | Out-Null
            $settled = $true
            break
        }
        Start-Sleep -Seconds 1
    }
    $ErrorActionPreference = $prevEap
    if ($settled) {
        Write-Step "OK   hive settled for $hiveFile"
    }
    else {
        Write-Step "WARN hive not settled within 45s: $hiveFile"
    }
}

function Set-RunUserHiveFrozen {
    param(
        [string]$ProfileRoot,
        [string]$User
    )
    $hiveFile = Join-Path $ProfileRoot 'NTUSER.DAT'
    if (-not (Test-Path -LiteralPath $hiveFile)) {
        Write-Step "WARN NTUSER.DAT 不存在（$User 未登录过）——HKCU 冻结依赖 profile ACL 文件层兜底；请先以 $User 登录一次后重跑本脚本"
        return
    }
    $mount = "gsa_hkcu_$([guid]::NewGuid().ToString('N'))"
    $prevEap = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    $loaded = $false
    try {
        # A LoadUserProfileW/UnloadUserProfileW (or reg load/unload) that
        # finished moments earlier can leave NTUSER.DAT transiently locked;
        # retry the freeze mount before failing the arm.
        $deadline = (Get-Date).AddSeconds(45)
        while ((Get-Date) -lt $deadline) {
            & reg.exe load "HKU\$mount" $hiveFile 2>&1 | Out-Null
            if ($LASTEXITCODE -eq 0) {
                $loaded = $true
                break
            }
            Start-Sleep -Seconds 2
        }
    }
    finally {
        $ErrorActionPreference = $prevEap
    }
    if (-not $loaded) {
        Write-Step "FAIL reg load NTUSER.DAT（HKCU 冻结未生效，需先确保 $User 未登录）"
        $script:StepFailures++
        return
    }
    try {
        Invoke-RegistryDeny "Registry::HKEY_USERS\$mount" $User
    }
    finally {
        [GC]::Collect()
        [GC]::WaitForPendingFinalizers()
        $prevEap = $ErrorActionPreference
        $ErrorActionPreference = 'Continue'
        try {
            & reg.exe unload "HKU\$mount" 2>&1 | Out-Null
            if ($LASTEXITCODE -ne 0) {
                & reg.exe unload "HKU\$mount" 2>&1 | Out-Null
                Write-Step "WARN reg unload $mount 失败（重试一次）"
            }
        }
        finally {
            $ErrorActionPreference = $prevEap
        }
        Wait-RunUserHiveSettled $ProfileRoot
    }
}

function Clear-RunUserHiveFrozen {
    param(
        [string]$ProfileRoot,
        [string]$User
    )
    $hiveFile = Join-Path $ProfileRoot 'NTUSER.DAT'
    if (-not (Test-Path -LiteralPath $hiveFile)) {
        return
    }
    $mount = "gsa_hkcu_$([guid]::NewGuid().ToString('N'))"
    $prevEap = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try {
        & reg.exe load "HKU\$mount" $hiveFile 2>&1 | Out-Null
        if ($LASTEXITCODE -ne 0) {
            $ErrorActionPreference = $prevEap
            return
        }
        try {
            Invoke-RegistryDenyRemove "Registry::HKEY_USERS\$mount" $User
        }
        finally {
            [GC]::Collect()
            [GC]::WaitForPendingFinalizers()
            & reg.exe unload "HKU\$mount" 2>&1 | Out-Null
            Wait-RunUserHiveSettled $ProfileRoot
        }
    }
    finally {
        $ErrorActionPreference = $prevEap
    }
}

function Set-FirewallEgressPolicy {
    param([switch]$Restore)
    $manifest = Join-Path $PSScriptRoot "logs\firewall-rules-$Arm.txt"
    $manifestDir = Split-Path -Parent $manifest
    if ($manifestDir -and -not (Test-Path -LiteralPath $manifestDir)) {
        New-Item -ItemType Directory -Path $manifestDir -Force | Out-Null
    }
    $hostsPath = "$env:SystemRoot\System32\drivers\etc\hosts"
    $hostsBackup = "$hostsPath.gsa-before-hardening"
    if ($Restore) {
        if ($PSCmdlet.ShouldProcess('firewall', 'restore outbound default allow')) {
            & netsh advfirewall set allprofiles firewallpolicy blockinbound,allowoutbound 2>&1 | Out-Null
            if (Test-Path -LiteralPath $manifest) {
                Get-Content -LiteralPath $manifest | ForEach-Object {
                    $ruleName = $_.Trim()
                    if ($ruleName) {
                        & netsh advfirewall firewall delete rule name="$ruleName" 2>&1 | Out-Null
                    }
                }
                Remove-Item -LiteralPath $manifest -Force
                Write-Step "OK   firewall rules from manifest removed ($manifest)"
            }
            else {
                & netsh advfirewall firewall delete rule name="$RulePrefix-BLOCK-METADATA" 2>&1 | Out-Null
                & netsh advfirewall firewall delete rule name="$RulePrefix-ALLOW" 2>&1 | Out-Null
            }
            if (Test-Path -LiteralPath $hostsBackup) {
                Copy-Item -LiteralPath $hostsBackup -Destination $hostsPath -Force
                Write-Step "OK   hosts restored"
            }
            Write-Step "OK   firewall egress policy restored"
        }
        return
    }
    if ($PSCmdlet.ShouldProcess('firewall', 'blockoutbound default + rules')) {
        & netsh advfirewall set allprofiles state on 2>&1 | Out-Null
        & netsh advfirewall set allprofiles firewallpolicy blockinbound,blockoutbound 2>&1 | Out-Null
        & netsh advfirewall firewall add rule name="$RulePrefix-BLOCK-METADATA" dir=out action=block `
            profile=any enable=yes remoteip="169.254.169.254,169.254.0.0/16,10.0.0.0/8,172.16.0.0/12,192.168.0.0/16,224.0.0.0/4,ff00::/8,fe80::/10,fc00::/7" 2>&1 | Out-Null
        Add-Content -LiteralPath $manifest -Value "$RulePrefix-BLOCK-METADATA" -Encoding ascii
        Write-Step "OK   firewall default outbound = block; metadata/link-local/RFC1918/multicast block rule"
        foreach ($ip in $AllowlistIps) {
            & netsh advfirewall firewall add rule name="$RulePrefix-ALLOW-$ip" dir=out action=allow `
                profile=any enable=yes remoteip=$ip 2>&1 | Out-Null
            Add-Content -LiteralPath $manifest -Value "$RulePrefix-ALLOW-$ip" -Encoding ascii
            Write-Step "OK   firewall allow $ip"
        }
        # DeepSeek 模型端点恒放行（设计 §3 Network 行）：优先显式 -DeepSeekIp，
        # 否则必须在模板有网时解析钉 IP；解析不到则本次加固 FAIL（不变量不满足）。
        if (Test-Path -LiteralPath $hostsPath) {
            Copy-Item -LiteralPath $hostsPath -Destination $hostsBackup -Force
        }
        $deepseek = @()
        if ($DeepSeekIp) {
            $deepseek = @($DeepSeekIp)
        }
        else {
            try {
                $deepseek = Resolve-DnsName api.deepseek.com -Type A -ErrorAction SilentlyContinue |
                    Where-Object { $_.Type -eq 'A' } | Select-Object -ExpandProperty IPAddress -Unique
            }
            catch { }
        }
        foreach ($ip in $deepseek) {
            & netsh advfirewall firewall add rule name="$RulePrefix-ALLOW-DEEPSEEK-$ip" dir=out action=allow `
                profile=any enable=yes remoteip=$ip 2>&1 | Out-Null
            Add-Content -LiteralPath $manifest -Value "$RulePrefix-ALLOW-DEEPSEEK-$ip" -Encoding ascii
            $line = "$ip api.deepseek.com"
            if (-not (Select-String -LiteralPath $hostsPath -Pattern 'api\.deepseek\.com' -Quiet)) {
                Add-Content -LiteralPath $hostsPath -Value $line -Encoding ascii
            }
            Write-Step "OK   firewall allow DeepSeek $ip + hosts pin"
        }
        if (-not $deepseek) {
            Write-Step "FAIL DeepSeek 端点既未显式提供 -DeepSeekIp 也无法解析——'恒放行'不变量不满足，模板不得进入评测"
            $script:StepFailures++
        }
    }
}

function Set-AppLockerAllowlist {
    if (-not $PSCmdlet.ShouldProcess('AppLocker', 'apply whitelist policy')) {
        return
    }
    try {
        $xml = @"
<?xml version="1.0" encoding="utf-8"?>
<AppLockerPolicy Version="1">
  <RuleCollection Type="Exe" EnforcementMode="Enabled">
    <FilePathRule Id="$([guid]::NewGuid())" Name="Allow Windows" Description="" UserOrGroupSid="S-1-1-0" Action="Allow">
      <Conditions><FilePathCondition Path="%WINDIR%\*" /></Conditions>
    </FilePathRule>
    <FilePathRule Id="$([guid]::NewGuid())" Name="Allow Program Files" Description="" UserOrGroupSid="S-1-1-0" Action="Allow">
      <Conditions><FilePathCondition Path="%PROGRAMFILES%\*" /></Conditions>
    </FilePathRule>
    <FilePathRule Id="$([guid]::NewGuid())" Name="Allow Program Files x86" Description="" UserOrGroupSid="S-1-1-0" Action="Allow">
      <Conditions><FilePathCondition Path="%PROGRAMFILES(X86)%\*" /></Conditions>
    </FilePathRule>
    <FilePathRule Id="$([guid]::NewGuid())" Name="Allow Workspace" Description="" UserOrGroupSid="S-1-1-0" Action="Allow">
      <Conditions><FilePathCondition Path="$WorkspaceRoot\*" /></Conditions>
    </FilePathRule>
  </RuleCollection>
</AppLockerPolicy>
"@
        $policyXml = Join-Path $env:TEMP "gsa-applocker-$Arm-$([guid]::NewGuid().ToString('N')).xml"
        Set-Content -LiteralPath $policyXml -Value $xml -Encoding utf8
        Set-AppLockerPolicy -XmlPolicy $policyXml
        Write-Step "OK   AppLocker whitelist policy applied ($policyXml)"
    }
    catch {
        Write-Step "FAIL AppLocker policy : $($_.Exception.Message)（需 Pro/Enterprise 且组件可用）"
        $script:StepFailures++
    }
}

function Reset-AppLockerPolicy {
    if ($PSCmdlet.ShouldProcess('AppLocker', 'reset policy')) {
        try {
            Set-AppLockerPolicy -Policy $null -ErrorAction Stop
            Write-Step "OK   AppLocker policy reset"
        }
        catch {
            # Some builds reject -Policy $null at parameter validation.
            # Fallback: delete the local SrpV2 policy key (equivalent reset;
            # verified path from the 2026-09-02 triage).
            try {
                $srp = 'HKLM:\SOFTWARE\Policies\Microsoft\Windows\SrpV2'
                if (Test-Path -LiteralPath $srp) {
                    Remove-Item -LiteralPath $srp -Recurse -Force -ErrorAction Stop
                }
                Write-Step "OK   AppLocker policy reset (fallback: SrpV2 removed)"
            }
            catch {
                Write-Step "FAIL AppLocker reset : $($_.Exception.Message)"
                $script:StepFailures++
            }
        }
    }
}

function Get-ProtectedPaths {
    $paths = @(
        "$env:SystemRoot",
        "$env:ProgramFiles",
        "${env:ProgramFiles(x86)}",
        "$env:ProgramData"
    ) | Where-Object { $_ -and (Test-Path -LiteralPath $_) } | Select-Object -Unique
    if ($Arm -eq 'high-nist') {
        # S4 正式序列修复：脚本常以 SYSTEM 运行，$env:USERPROFILE 是
        # C:\WINDOWS\system32\config\systemprofile，推导会把 RunUser profile
        # 根错指到 C:\WINDOWS\system32\config\<RunUser>（干净基线上不存在，
        # ACL deny 直接 FAIL）。统一走 Get-RunUserProfileRoot 的固定路径
        # %SystemDrive%\Users\<RunUser>。
        $profileRoot = Get-RunUserProfileRoot $RunUser
        $paths += @(
            $profileRoot,
            (Join-Path $profileRoot 'AppData\Roaming'),
            (Join-Path $profileRoot 'AppData\Local')
        )
    }
    # 盘符根：只拒绝根目录本身的写（不传播继承，避免误伤工作区）。
    # 仅 NTFS 固定盘可做 ACL（排除 ISO/UDF 光驱与 FAT32/EFI 分区）。
    $driveRoots = Get-PSDrive -PSProvider FileSystem |
        Where-Object { $_.Free -ne $null } |
        ForEach-Object {
            $vol = Get-Volume -DriveLetter $_.Name -ErrorAction SilentlyContinue
            if ($vol -and $vol.FileSystem -eq 'NTFS') { "$($_.Root)" }
        }
    return @{ tree = $paths; roots = $driveRoots }
}

Assert-Admin
Write-Step "=== apply_hardening.ps1 Arm=$Arm Revert=$Revert RunUser=$RunUser ==="

if ($Arm -eq 'control') {
    if ($Revert) {
        Write-Step "control 臂无策略可撤销"
    }
    else {
        Write-Step "control 臂为全开放基线，不应用任何策略"
    }
}
else {
    Ensure-RunUser
    $profileRootForUser = ''
    if ($Arm -eq 'high-nist') {
        # 先确保 RunUser profile 目录存在（New-LocalUser 不创建 profile），
        # 否则后续 icacls deny 会因目标不存在而失败。
        $profileRootForUser = Get-RunUserProfileRoot $RunUser
        Ensure-RunUserProfileDirectories $profileRootForUser
    }

    # --- UAC / 提权路径收紧（Privilege·non-admin） ---
    $systemPolicy = 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\System'
    if ($Revert) {
        Write-Step "UAC 策略保持系统默认（Revert 不动 EnableLUA）"
    }
    else {
        Set-RegistryDword $systemPolicy 'EnableLUA' 1
        Set-RegistryDword $systemPolicy 'ConsentPromptBehaviorAdmin' 2
        Set-RegistryDword $systemPolicy 'PromptOnSecureDesktop' 1
        Set-RegistryDword $systemPolicy 'EnableInstallerDetection' 1
        Set-RegistryDword $systemPolicy 'FilterAdministratorToken' 1
        $installer = 'HKLM:\SOFTWARE\Policies\Microsoft\Windows\Installer'
        Set-RegistryDword $installer 'EnableUserControl' 0
    }
    if ($DisableBreakGlassAdmin -and -not $Revert) {
        if ($PSCmdlet.ShouldProcess('Administrator', 'disable built-in account')) {
            Disable-LocalUser -Name 'Administrator' -ErrorAction SilentlyContinue
            Write-Step "OK   built-in Administrator disabled"
        }
    }

    # --- Filesystem·readonly-os + freeze-home ---
    $protected = Get-ProtectedPaths
    if ($Revert) {
        foreach ($path in $protected.tree) {
            Invoke-AclDenyRemove $path $RunUser
        }
        foreach ($root in $protected.roots) {
            Invoke-AclDenyRemove $root $RunUser ''
        }
        Invoke-RegistryDenyRemove 'HKLM:\SOFTWARE' $RunUser
        if ($Arm -eq 'high-nist') {
            Invoke-RegistryDenyRemove 'Registry::HKEY_CLASSES_ROOT' $RunUser
            # 旧版误加到管理员 HKCU 的 ACE 也一并清掉（向后兼容）。
            Invoke-RegistryDenyRemove 'Registry::HKEY_CURRENT_USER' $RunUser
            Clear-RunUserHiveFrozen $profileRootForUser $RunUser
            $packages = Join-Path $profileRootForUser 'AppData\Local\Packages'
            if (Test-Path -LiteralPath $packages) {
                Invoke-AclGrantRemove $packages $RunUser
            }
        }
    }
    else {
        foreach ($path in $protected.tree) {
            Invoke-AclDeny $path $RunUser
        }
        foreach ($root in $protected.roots) {
            Invoke-AclDeny $root $RunUser ''
        }
        Invoke-RegistryDeny 'HKLM:\SOFTWARE' $RunUser
        if ($Arm -eq 'high-nist') {
            Invoke-RegistryDeny 'Registry::HKEY_CLASSES_ROOT' $RunUser
            # HKCU 冻结必须作用在 RunUser 自己的 hive（NTUSER.DAT），
            # 管理员会话里的 HKEY_CURRENT_USER 是管理员的 hive，加 ACE 无效。
            Set-RunUserHiveFrozen $profileRootForUser $RunUser
            # AppContainer profile 由系统建在 AppData\Local\Packages，
            # high-nist 仍需该目录可写；显式 grant 优先于继承 deny。
            $packages = Join-Path $profileRootForUser 'AppData\Local\Packages'
            if (-not (Test-Path -LiteralPath $packages)) {
                New-Item -ItemType Directory -Path $packages -Force | Out-Null
            }
            Invoke-AclGrant $packages $RunUser
        }
    }

    # --- 工作区 + TEMP 重定向目录（高摩擦下 orz 自身仍需可写临时目录） ---
    if ($PSCmdlet.ShouldProcess($WorkspaceRoot, 'ensure workspace + .tmp')) {
        try {
            New-Item -ItemType Directory -Path $WorkspaceRoot -Force | Out-Null
            New-Item -ItemType Directory -Path (Join-Path $WorkspaceRoot '.tmp') -Force | Out-Null
            if ($Revert) {
                Invoke-AclGrantRemove $WorkspaceRoot $RunUser
            }
            else {
                Invoke-AclGrant $WorkspaceRoot $RunUser
            }
            Write-Step "OK   workspace $WorkspaceRoot ready (RunUser=M)"
        }
        catch {
            Write-Step "FAIL workspace $WorkspaceRoot : $($_.Exception.Message)"
            $script:StepFailures++
        }
    }
}

if ($Arm -eq 'high-nist') {
    if ($Revert) {
        Set-FirewallEgressPolicy -Restore
        if ($EnableAppLocker) {
            Reset-AppLockerPolicy
        }
    }
    else {
        Set-FirewallEgressPolicy
        if ($EnableAppLocker) {
            Set-AppLockerAllowlist
        }
    }
}

Write-Step "=== done (StepFailures=$script:StepFailures) ==="
if ($script:StepFailures -ne 0) {
    Write-Error "apply_hardening.ps1 有 $script:StepFailures 个步骤失败，详见 $LogPath"
    exit 1
}
Write-Step "apply_hardening.ps1 $Arm OK"
