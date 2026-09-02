$out = 'D:\CLI\_windows_high_nist\vm-tpm5.txt'
$lines = @()
try {
    if ((Get-VM -Name 'win-s4').State -ne 'Off') {
        Stop-VM -Name 'win-s4' -Force
        Start-Sleep -Seconds 2
    }
    Set-VMKeyProtector -VMName 'win-s4' -NewLocalKeyProtector -ErrorAction Stop
    $lines += "KeyProtector set (local)"
}
catch {
    $lines += "KP_ERR: $($_.Exception.Message)"
}
try {
    Enable-VMTPM -VMName 'win-s4' -ErrorAction Stop
    $lines += "TPM enabled"
}
catch {
    $lines += "TPM_ERR: $($_.Exception.Message)"
}
try {
    Start-VM -Name 'win-s4' -ErrorAction Stop
    Start-Sleep -Seconds 3
    $lines += "State=$((Get-VM -Name 'win-s4').State)"
}
catch {
    $lines += "START_ERR: $($_.Exception.Message)"
}
$lines | Set-Content -LiteralPath $out -Encoding utf8
