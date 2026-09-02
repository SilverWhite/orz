<#
    S4 dedicated VM creation script (run as Administrator).
    Idempotent: an existing VM named win-s4 is removed and recreated.
#>
[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$script:StepFailures = 0

$vmName = 'win-s4'
$vmDir = 'D:\VMs\win-s4'
$vhdPath = Join-Path $vmDir 'win-s4.vhdx'
$isoPath = 'D:\VMs\iso\26200.6584.250915-1905.25h2_ge_release_svc_refresh_CLIENTENTERPRISEEVAL_OEMRET_x64FRE_zh-cn.iso'
$memBytes = 6GB
$vhdSize = 80GB
$cpuCount = 4

function Write-Step {
    param([string]$Message)
    Write-Host "[$(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')] $Message"
}

# 0) Require administrator
$id = [System.Security.Principal.WindowsIdentity]::GetCurrent()
$principal = [System.Security.Principal.WindowsPrincipal]::new($id)
if (-not $principal.IsInRole([System.Security.Principal.WindowsBuiltInRole]::Administrator)) {
    throw 'Administrator rights required'
}

# 1) Directories and ISO
New-Item -ItemType Directory -Path $vmDir -Force | Out-Null
if (-not (Test-Path -LiteralPath $isoPath)) {
    throw "ISO not found: $isoPath"
}
Write-Step "OK ISO: $isoPath ($([math]::Round((Get-Item -LiteralPath $isoPath).Length/1GB,2)) GB)"

# 2) Virtual switch: prefer External (management OS shared), else Default Switch
$switchName = ''
try {
    $external = Get-VMSwitch | Where-Object SwitchType -eq 'External' | Select-Object -First 1
    if ($external) {
        $switchName = $external.Name
        Write-Step "OK using external switch: $switchName"
    }
} catch { }
if (-not $switchName) {
    try {
        $def = Get-VMSwitch -Name 'Default Switch' -ErrorAction SilentlyContinue
        if ($def) {
            $switchName = $def.Name
            Write-Step "OK using Default Switch: $switchName"
        }
    } catch { }
}
if (-not $switchName) {
    $adapter = Get-NetAdapter | Where-Object Status -eq 'Up' | Select-Object -First 1
    if (-not $adapter) {
        throw 'No network adapter available to create a switch'
    }
    $switchName = 'S4External'
    New-VMSwitch -Name $switchName -NetAdapterName $adapter.Name -AllowManagementOS $true
    Write-Step "OK created external switch: $switchName ($($adapter.Name))"
}

# 3) Recreate the VM
$existing = Get-VM -Name $vmName -ErrorAction SilentlyContinue
if ($existing) {
    if ($existing.State -ne 'Off') {
        Stop-VM -Name $vmName -Force
    }
    Remove-VM -Name $vmName -Force
    Write-Step "OK removed previous VM $vmName"
}

$vm = New-VM -Name $vmName `
    -MemoryStartupBytes $memBytes `
    -NewVHDPath $vhdPath `
    -NewVHDSizeBytes $vhdSize `
    -Generation 2 `
    -SwitchName $switchName
Set-VM -Name $vmName -ProcessorCount $cpuCount
Set-VMMemory -VMName $vmName -DynamicMemoryEnabled $false
Write-Step "OK VM created: $vmName (Gen2, ${cpuCount} vCPU, $([math]::Round($memBytes/1GB,0))GB RAM, VHDX $([math]::Round($vhdSize/1GB,0))GB dynamic)"

# 4) Attach ISO and set boot order (DVD first)
$dvd = Get-VMDvdDrive -VMName $vmName -ErrorAction SilentlyContinue
if (-not $dvd) {
    Add-VMDvdDrive -VMName $vmName -Path $isoPath
    $dvd = Get-VMDvdDrive -VMName $vmName
} else {
    Set-VMDvdDrive -VMName $vmName -ControllerNumber $dvd.ControllerNumber `
        -ControllerLocation $dvd.ControllerLocation -Path $isoPath
}
Set-VMFirmware -VMName $vmName -FirstBootDevice $dvd
Write-Step "OK ISO attached and set as first boot device"

Write-Step "=== done (StepFailures=$script:StepFailures) ==="
if ($script:StepFailures -ne 0) {
    exit 1
}
Write-Step "VM $vmName ready (not started)"
