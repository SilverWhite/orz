$out = 'D:\CLI\_windows_high_nist\vm-tpm4.txt'
$lines = @()
try {
    $tpm = Get-Tpm -ErrorAction Stop
    $lines += "TpmPresent=$($tpm.TpmPresent)"
    $lines += "TpmReady=$($tpm.TpmReady)"
    $lines += "TpmEnabled=$($tpm.TpmEnabled)"
}
catch {
    $lines += "TPM_ERR: $($_.Exception.Message)"
}
try {
    $kp = Get-Command -Name 'Set-VMKeyProtector' -ErrorAction Stop
    $lines += "Set-VMKeyProtector available: $($kp.Name)"
}
catch {
    $lines += "Set-VMKeyProtector unavailable"
}
$lines | Set-Content -LiteralPath $out -Encoding utf8
