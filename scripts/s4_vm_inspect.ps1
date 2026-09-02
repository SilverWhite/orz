<#
    S4 VM inspect helper (run elevated). Writes diagnostics to OutFile.
    ASCII only.
#>
[CmdletBinding()]
param(
    [string]$OutFile = 'D:\CLI\_windows_high_nist\vm-inspect2.txt'
)

$ErrorActionPreference = 'Continue'
$lines = New-Object System.Collections.Generic.List[string]

$vm = Get-VM -Name 'win-s4' -ErrorAction SilentlyContinue
if ($vm) {
    $lines.Add("STATE=$($vm.State)")
    $lines.Add("UPTIME=$($vm.Uptime)")
    $net = Get-VMNetworkAdapter -VMName 'win-s4'
    foreach ($ad in $net) {
        $lines.Add("ADAPTER name=$($ad.Name) mac=$($ad.MacAddress) connected=$($ad.Connected) ips=$($ad.IPAddresses -join ',')")
    }
} else {
    $lines.Add("VM_NOT_FOUND")
}

try {
    $sec = ConvertTo-SecureString '123456' -AsPlainText -Force
    $cred = New-Object System.Management.Automation.PSCredential('HL', $sec)
    $res = Invoke-Command -VMName 'win-s4' -Credential $cred -ScriptBlock {
        $os = Get-CimInstance Win32_OperatingSystem
        $id = [System.Security.Principal.WindowsIdentity]::GetCurrent()
        $p = [System.Security.Principal.WindowsPrincipal]::new($id)
        $ip = (Get-NetIPAddress -AddressFamily IPv4 | Where-Object { $_.IPAddress -notlike '127.*' -and $_.IPAddress -notlike '169.254.*' } | Select-Object -ExpandProperty IPAddress) -join ','
        $net = Test-NetConnection -ComputerName 'api.deepseek.com' -Port 443 -WarningAction SilentlyContinue
        [pscustomobject]@{
            OS = $os.Caption
            Version = $os.Version
            Build = $os.BuildNumber
            Computer = $env:COMPUTERNAME
            User = $id.Name
            IsAdmin = $p.IsInRole([System.Security.Principal.WindowsBuiltInRole]::Administrator)
            IP = $ip
            DeepSeekTcp443 = $net.TcpTestSucceeded
        } | Format-List | Out-String
    } -ErrorAction Stop
    $lines.Add("PSDIRECT_OK")
    $lines.Add(($res | Out-String).Trim())
} catch {
    $lines.Add("PSDIRECT_ERR: $($_.Exception.Message)")
}

$lines | Set-Content -LiteralPath $OutFile -Encoding utf8
