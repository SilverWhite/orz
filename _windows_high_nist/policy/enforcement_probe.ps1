<#
    enforcement_probe.ps1 — Windows enforcement-probe（无 agent 先验墙，
    P0-0l-②，等效 Linux 干跑的 policy/enforcement_probe.sh）。

    用法（在目标墙环境内运行——硬化 VM 会话，或经
    run_windows_native_sandbox_command.py --arm <arm> 拉起）：
      .\enforcement_probe.ps1 -Arm high-nist -Workspace D:\workspace

    每轴断言（7 项 GAK-SBX-001 检查扩展为每轴集合）：
      control    — workspace_writable, os_writable
      non-admin  — workspace_writable, non_admin, token_virtualization_disabled,
                   system32_write_blocked, program_files_write_blocked,
                   hklm_write_blocked, runas_blocked, privileges_removed,
                   temp_write_succeeded, home_write_succeeded
      high-nist  — 上述非 admin 全部 + programdata_write_blocked,
                   drive_root_write_blocked, hkcufrozen, home_frozen,
                   appdata_frozen, temp_redirected, low_integrity,
                   appcontainer_token, job_object_assigned, network_blocked,
                   metadata_blocked, probe_file_cleaned

    退出码：0=墙全部生效；非 0=某断言失败（fail-closed，该臂不作数）。
    -ResultPath 可选：把断言结果写为 JSON。
#>

[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidateSet('control', 'non-admin', 'high-nist')]
    [string]$Arm,

    [string]$Workspace = '',

    [string]$TempDir = '',

    [string]$ResultPath = '',

    [string]$AllowlistProbeIp = '1.1.1.1',

    [string]$AllowlistReachabilityIp = ''
)

$ErrorActionPreference = 'Continue'
$script:Failures = 0
$script:Checks = @{}

if (-not $Workspace) {
    $Workspace = $env:GSA_PROBE_WORKSPACE
}
if (-not $Workspace) {
    $Workspace = (Get-Location).Path
}
if (-not $TempDir) {
    $TempDir = $env:GSA_PROBE_TEMPDIR
}

# P/Invoke：IsProcessInJob（Job Object containment 断言）。
$jobHelperSource = @'
using System;
using System.Runtime.InteropServices;
public static class GsaJobProbe {
    [DllImport("kernel32.dll", SetLastError = true)]
    public static extern IntPtr GetCurrentProcess();
    [DllImport("kernel32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool IsProcessInJob(IntPtr hProcess, IntPtr hJob, out bool result);
    public static bool InAnyJob() {
        bool result;
        if (!IsProcessInJob(GetCurrentProcess(), IntPtr.Zero, out result)) {
            return false;
        }
        return result;
    }
}
'@
$privHelperSource = @'
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;
public static class GsaPrivilegeProbe {
    [DllImport("kernel32.dll", SetLastError = true)]
    public static extern IntPtr GetCurrentProcess();
    [DllImport("advapi32.dll", SetLastError = true)]
    public static extern bool OpenProcessToken(IntPtr h, uint access, out IntPtr token);
    [DllImport("advapi32.dll", SetLastError = true)]
    public static extern bool GetTokenInformation(IntPtr token, int cls, IntPtr info, uint len, out uint needed);
    [DllImport("advapi32.dll", SetLastError = true)]
    public static extern bool LookupPrivilegeName(string host, ref Luid luid, StringBuilder name, ref uint size);
    [DllImport("kernel32.dll", SetLastError = true)]
    public static extern bool CloseHandle(IntPtr h);
    [StructLayout(LayoutKind.Sequential)]
    public struct Luid { public uint LowPart; public int HighPart; }
    [StructLayout(LayoutKind.Sequential)]
    public struct LuidAndAttributes { public Luid Luid; public uint Attributes; }
    [StructLayout(LayoutKind.Sequential)]
    public struct TokenPrivileges { public uint PrivilegeCount; public LuidAndAttributes Privileges; }
    public static string[] EnabledPrivileges() {
        var list = new List<string>();
        IntPtr token;
        if (!OpenProcessToken(GetCurrentProcess(), 0x0008, out token)) {
            return list.ToArray();
        }
        try {
            uint needed;
            if (!GetTokenInformation(token, 3, IntPtr.Zero, 0, out needed)) {
                int err = Marshal.GetLastWin32Error();
                if (err != 122) { return list.ToArray(); }
            }
            IntPtr buf = Marshal.AllocHGlobal((int)needed);
            try {
                if (!GetTokenInformation(token, 3, buf, needed, out needed)) {
                    return list.ToArray();
                }
                uint count = (uint)Marshal.ReadInt32(buf, 0);
                int offset = (int)Marshal.OffsetOf(typeof(TokenPrivileges), "Privileges");
                int stride = Marshal.SizeOf(typeof(LuidAndAttributes));
                for (uint i = 0; i < count; i++) {
                    IntPtr p = new IntPtr(buf.ToInt64() + offset + i * stride);
                    LuidAndAttributes la = (LuidAndAttributes)Marshal.PtrToStructure(p, typeof(LuidAndAttributes));
                    if ((la.Attributes & 0x2) == 0) { continue; }
                    uint size = 0;
                    Luid luid = la.Luid;
                    LookupPrivilegeName(null, ref luid, null, ref size);
                    if (size == 0) { continue; }
                    var sb = new StringBuilder((int)size);
                    if (LookupPrivilegeName(null, ref luid, sb, ref size)) {
                        list.Add(sb.ToString());
                    }
                }
            }
            finally { Marshal.FreeHGlobal(buf); }
        }
        finally { CloseHandle(token); }
        return list.ToArray();
    }
}
'@
try {
    Add-Type -TypeDefinition $jobHelperSource -ErrorAction Stop
    Add-Type -TypeDefinition $privHelperSource -ErrorAction Stop
}
catch {
    Write-Warning "Add-Type 失败（IsProcessInJob/特权枚举）：$($_.Exception.Message)"
}

function Assert-Check {
    param(
        [string]$Name,
        [bool]$Condition
    )
    $script:Checks[$Name] = [bool]$Condition
    if ($Condition) {
        Write-Output "PASS  $Name"
    }
    else {
        Write-Output "FAIL  $Name"
        $script:Failures++
    }
}

function Test-PathWriteBlocked {
    param([string]$Path)
    $target = Join-Path $Path ".p2-probe-w-$(Get-Random).tmp"
    try {
        Set-Content -LiteralPath $target -Value 'x' -ErrorAction Stop
        Remove-Item -LiteralPath $target -Force -ErrorAction SilentlyContinue
        return $false
    }
    catch {
        return $true
    }
}

function Test-PathWriteSucceeded {
    param([string]$Path)
    $target = Join-Path $Path ".p2-probe-ok-$(Get-Random).tmp"
    try {
        Set-Content -LiteralPath $target -Value 'x' -ErrorAction Stop
        Remove-Item -LiteralPath $target -Force -ErrorAction SilentlyContinue
        return $true
    }
    catch {
        return $false
    }
}

function Test-RegistryWriteBlocked {
    param([string]$KeyPath)
    $probe = Join-Path $KeyPath '_p2_ps_probe_del'
    try {
        New-Item -Path $probe -Force -ErrorAction Stop | Out-Null
        Remove-Item -Path $probe -Force -ErrorAction SilentlyContinue
        return $false
    }
    catch {
        return $true
    }
}

function Test-TcpBlocked {
    param(
        [string]$Ip,
        [int]$Port,
        [int]$TimeoutMs = 1500
    )
    try {
        $tcp = [System.Net.Sockets.TcpClient]::new()
        try {
            $ar = $tcp.BeginConnect($Ip, $Port, $null, $null)
            $connected = $ar.AsyncWaitHandle.WaitOne($TimeoutMs, $false) -and $tcp.Connected
            if (-not $tcp.Connected) { try { $tcp.EndConnect($ar) } catch { } }
            return -not $connected
        }
        finally {
            $tcp.Close()
        }
    }
    catch {
        return $true
    }
}

function Get-IntegrityLevel {
    $groups = & whoami /groups 2>$null
    foreach ($line in $groups) {
        if ($line -match 'S-1-16-(\d+)') {
            return [int]$Matches[1]
        }
    }
    return $null
}

function Test-AppContainerToken {
    $groups = & whoami /groups 2>$null
    foreach ($line in $groups) {
        if ($line -match 'S-1-15-2-\d+') {
            return $true
        }
    }
    return $false
}

function Get-EnabledPrivileges {
    # 主路径用 P/Invoke 枚举 token 启用特权（与输出本地化无关）；
    # whoami 文本解析仅作 fallback（其 Enabled 状态列可能随 locale 变化）。
    try {
        if ('GsaPrivilegeProbe' -as [type]) {
            return @([GsaPrivilegeProbe]::EnabledPrivileges())
        }
    }
    catch { }
    $privs = & whoami /priv 2>$null
    $enabled = @()
    foreach ($line in $privs) {
        if ($line -match '^\s*(Se\w+Privilege)\s+.*?(Enabled)\s*$') {
            $enabled += $Matches[1]
        }
    }
    return $enabled
}

function Test-AnyRestrictedPrivilege {
    $restricted = @(
        'SeDebugPrivilege',
        'SeBackupPrivilege',
        'SeRestorePrivilege',
        'SeTakeOwnershipPrivilege',
        'SeLoadDriverPrivilege',
        'SeCreateSymbolicLinkPrivilege'
    )
    $enabled = Get-EnabledPrivileges
    foreach ($name in $restricted) {
        if ($enabled -contains $name) {
            return $true
        }
    }
    return $false
}

function Test-IsAdmin {
    try {
        $identity = [System.Security.Principal.WindowsIdentity]::GetCurrent()
        $principal = [System.Security.Principal.WindowsPrincipal]::new($identity)
        return $principal.IsInRole([System.Security.Principal.WindowsBuiltInRole]::Administrator)
    }
    catch {
        return $true
    }
}

# --- 通用：工作区可写（三臂都要求） ---
$wsOk = Test-PathWriteSucceeded $Workspace
Assert-Check 'workspace_writable' $wsOk

switch ($Arm) {
    'control' {
        # control 臂只要求工作区可写 + OS 面可写。
        $osTarget = Join-Path $env:SystemRoot 'Temp'
        if (-not (Test-Path -LiteralPath $osTarget)) {
            $osTarget = $env:ProgramData
        }
        Assert-Check 'os_writable' (Test-PathWriteSucceeded $osTarget)
    }

    'non-admin' {
        Assert-Check 'non_admin' (-not (Test-IsAdmin))

        # token 虚拟化静默写检测：写 Program Files 应失败，且不得落 VirtualStore。
        $pfWrite = Test-PathWriteBlocked $env:ProgramFiles
        $virtualStore = Join-Path $env:LOCALAPPDATA 'VirtualStore'
        $virtualized = $false
        if (Test-Path -LiteralPath $virtualStore) {
            $probeName = Get-ChildItem -LiteralPath $virtualStore -Recurse -Filter '.p2-probe-w-*.tmp' `
                -ErrorAction SilentlyContinue | Select-Object -First 1
            $virtualized = $null -ne $probeName
        }
        Assert-Check 'token_virtualization_disabled' ($pfWrite -and -not $virtualized)
        Assert-Check 'program_files_write_blocked' $pfWrite
        Assert-Check 'system32_write_blocked' (Test-PathWriteBlocked (Join-Path $env:SystemRoot 'System32'))
        Assert-Check 'hklm_write_blocked' (Test-RegistryWriteBlocked 'HKLM:\SOFTWARE')
        Assert-Check 'privileges_removed' (-not (Test-AnyRestrictedPrivilege))

        # runas/提权路径收紧（UAC 策略 + 非管理员身份）。
        $uacOk = $false
        try {
            $enableLua = (Get-ItemProperty -LiteralPath 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\System' -Name EnableLUA -ErrorAction Stop).EnableLUA
            $consent = (Get-ItemProperty -LiteralPath 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\System' -Name ConsentPromptBehaviorAdmin -ErrorAction Stop).ConsentPromptBehaviorAdmin
            $uacOk = ($enableLua -eq 1) -and ($consent -eq 2)
        }
        catch { }
        Assert-Check 'runas_blocked' ($uacOk -and -not (Test-IsAdmin))
        Assert-Check 'temp_write_succeeded' (Test-PathWriteSucceeded $env:TEMP)
        Assert-Check 'home_write_succeeded' (Test-PathWriteSucceeded $env:USERPROFILE)
    }

    'high-nist' {
        Assert-Check 'non_admin' (-not (Test-IsAdmin))

        $pfWrite = Test-PathWriteBlocked $env:ProgramFiles
        $virtualStore = Join-Path $env:LOCALAPPDATA 'VirtualStore'
        $virtualized = $false
        if (Test-Path -LiteralPath $virtualStore) {
            $probeName = Get-ChildItem -LiteralPath $virtualStore -Recurse -Filter '.p2-probe-w-*.tmp' `
                -ErrorAction SilentlyContinue | Select-Object -First 1
            $virtualized = $null -ne $probeName
        }
        Assert-Check 'token_virtualization_disabled' ($pfWrite -and -not $virtualized)
        Assert-Check 'program_files_write_blocked' $pfWrite
        Assert-Check 'programdata_write_blocked' (Test-PathWriteBlocked $env:ProgramData)
        Assert-Check 'system32_write_blocked' (Test-PathWriteBlocked (Join-Path $env:SystemRoot 'System32'))

        $driveRoot = (Split-Path -Qualifier $env:SystemRoot) + '\'
        Assert-Check 'drive_root_write_blocked' (Test-PathWriteBlocked $driveRoot)
        Assert-Check 'hklm_write_blocked' (Test-RegistryWriteBlocked 'HKLM:\SOFTWARE')
        Assert-Check 'hkcufrozen' (Test-RegistryWriteBlocked 'HKCU:\SOFTWARE')
        Assert-Check 'home_frozen' (Test-PathWriteBlocked $env:USERPROFILE)
        Assert-Check 'appdata_frozen' (
            (Test-PathWriteBlocked $env:APPDATA) -and
            (Test-PathWriteBlocked $env:LOCALAPPDATA)
        )
        Assert-Check 'privileges_removed' (-not (Test-AnyRestrictedPrivilege))

        try {
            $enableLua = (Get-ItemProperty -LiteralPath 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\System' -Name EnableLUA -ErrorAction Stop).EnableLUA
            $consent = (Get-ItemProperty -LiteralPath 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\System' -Name ConsentPromptBehaviorAdmin -ErrorAction Stop).ConsentPromptBehaviorAdmin
            $uacOk = ($enableLua -eq 1) -and ($consent -eq 2)
        }
        catch { $uacOk = $false }
        Assert-Check 'runas_blocked' ($uacOk -and -not (Test-IsAdmin))

        # TEMP 重定向：env 指向 {workspace}\.tmp 且可写。
        $expectedTmp = if ($TempDir) { $TempDir } else { Join-Path $Workspace '.tmp' }
        $tempOk = $false
        if ($env:TEMP -and $env:TMP) {
            $tempOk = ($env:TEMP -like "$expectedTmp*") -and ($env:TMP -like "$expectedTmp*")
        }
        Assert-Check 'temp_redirected' ($tempOk -and (Test-PathWriteSucceeded $env:TEMP))

        $il = Get-IntegrityLevel
        Assert-Check 'low_integrity' ($null -ne $il -and $il -le 4096)
        Assert-Check 'appcontainer_token' (Test-AppContainerToken)
        $inJob = $false
        try { $inJob = [GsaJobProbe]::InAnyJob() } catch { }
        Assert-Check 'job_object_assigned' $inJob

        # blocked 探针 IP 必须避开 allowlist，否则该 IP 被放行后断言必 FAIL。
        $blockedProbeIp = $AllowlistProbeIp
        if ($AllowlistReachabilityIp -and $AllowlistReachabilityIp -eq $AllowlistProbeIp) {
            $fallback = @('8.8.8.8', '1.1.1.1') |
                Where-Object { $_ -ne $AllowlistReachabilityIp } |
                Select-Object -First 1
            if ($fallback) {
                $blockedProbeIp = $fallback
            }
        }
        Assert-Check 'network_blocked' (Test-TcpBlocked $blockedProbeIp 443)
        if ($AllowlistReachabilityIp) {
            Assert-Check 'allowlist_reachable' (-not (Test-TcpBlocked $AllowlistReachabilityIp 443))
        }
        Assert-Check 'metadata_blocked' (Test-TcpBlocked '169.254.169.254' 80)

        # 探针自清：除 result JSON（由调用方收集）外不留探针文件。
        $leftover = Get-ChildItem -LiteralPath $Workspace -Filter '.p2-probe-*.tmp' `
            -ErrorAction SilentlyContinue
        Assert-Check 'probe_file_cleaned' ($null -eq $leftover)
    }
}

if ($ResultPath) {
    $result = @{
        arm = $Arm
        workspace = $Workspace
        checks = $script:Checks
        passed = ($script:Checks.Values | Where-Object { $_ }).Count
        failed = $script:Failures
        ran_at = (Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ')
    }
    [System.IO.File]::WriteAllText(
        $ResultPath,
        ($result | ConvertTo-Json -Depth 4),
        [System.Text.UTF8Encoding]::new($false)
    )
}

if ($script:Failures -ne 0) {
    Write-Output "enforcement-probe: $Arm FAILED ($script:Failures assertions failed)"
    exit 1
}
Write-Output "enforcement-probe: $Arm OK"
