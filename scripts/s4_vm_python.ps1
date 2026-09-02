<#
    S4 VM python helper (run elevated): installs per-user Python 3.12 in the
    VM (official installer), verifies it, and tests the scheduled-task
    elevation channel (RunLevel Highest without interactive UAC).
    ASCII only.
#>
[CmdletBinding()]
param(
    [string]$OutFile = 'D:\CLI\_windows_high_nist\vm-python-result.txt'
)

$ErrorActionPreference = 'Stop'
$lines = New-Object System.Collections.Generic.List[string]

try {
    $sec = ConvertTo-SecureString '123456' -AsPlainText -Force
    $cred = New-Object System.Management.Automation.PSCredential('HL', $sec)
    $sess = New-PSSession -VMName 'win-s4' -Credential $cred

    # Ensure the staging directory exists, then stage the elevation probe.
    Invoke-Command -Session $sess -ScriptBlock {
        New-Item -ItemType Directory -Path 'C:\s4\tools' -Force | Out-Null
    }
    Copy-Item -LiteralPath 'D:\CLI\scripts\s4_elev_probe.ps1' `
        -Destination 'C:\s4\tools\s4_elev_probe.ps1' -ToSession $sess -Force
    Copy-Item -LiteralPath 'D:\CLI\_windows_high_nist\tools\python-3.12.10-amd64.exe' `
        -Destination 'C:\s4\tools\python-3.12.10-amd64.exe' -ToSession $sess -Force

    $guest = Invoke-Command -Session $sess -ScriptBlock {
        $out = New-Object System.Collections.Generic.List[string]
        $log = 'C:\s4\tools\py-install.log'
        Remove-Item -LiteralPath $log -Force -ErrorAction SilentlyContinue
        function Log([string]$m) {
            $out.Add($m)
            Add-Content -LiteralPath $log -Value $m -Encoding ascii
        }
        $installer = 'C:\s4\tools\python-3.12.10-amd64.exe'
        if (-not (Test-Path -LiteralPath $installer) -or (Get-Item -LiteralPath $installer).Length -lt 20000000) {
            Log "INSTALLER missing or corrupt"
            $out
            return
        }
        $installed = $false
        for ($attempt = 1; $attempt -le 10; $attempt++) {
            try {
                $p = Start-Process -FilePath $installer `
                    -ArgumentList @('/quiet', 'InstallAllUsers=0', 'PrependPath=1', 'Include_test=0', 'Include_launcher=1', 'AssociateFiles=0', 'Shortcuts=0') `
                    -Wait -PassThru -ErrorAction Stop
                Log "INSTALL attempt=$attempt exit=$($p.ExitCode)"
                $installed = ($p.ExitCode -eq 0)
                break
            } catch {
                Log "INSTALL attempt=$attempt err=$($_.Exception.Message)"
                Start-Sleep -Seconds 3
            }
        }
        if (-not $installed) {
            Log "INSTALL FAILED after retries"
        }

        $env:Path = [Environment]::GetEnvironmentVariable('Path', 'Machine') + ';' +
            [Environment]::GetEnvironmentVariable('Path', 'User')
        $pyOut = & python -c "import sys; print(sys.version)" 2>&1
        Log (("PY_VER exit=$LASTEXITCODE out=$($pyOut -join ' ')").Trim())
        $out
    }
    foreach ($l in $guest) { $lines.Add($l) }

    $elev = Invoke-Command -Session $sess -ScriptBlock {
        $out = New-Object System.Collections.Generic.List[string]
        $marker = 'C:\s4\tools\elev-test.txt'
        Remove-Item -LiteralPath $marker -Force -ErrorAction SilentlyContinue
        try {
            $taskName = 's4elevtest'
            Unregister-ScheduledTask -TaskName $taskName -Confirm:$false -ErrorAction SilentlyContinue
            $action = New-ScheduledTaskAction -Execute 'powershell.exe' `
                -Argument '-NoProfile -ExecutionPolicy Bypass -File C:\s4\tools\s4_elev_probe.ps1'
            $principal = New-ScheduledTaskPrincipal -UserId 'HL' -LogonType Password -RunLevel Highest
            $settings = New-ScheduledTaskSettingsSet -ExecutionTimeLimit (New-TimeSpan -Minutes 5) `
                -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries
            $task = New-ScheduledTask -Action $action -Principal $principal -Settings $settings
            Register-ScheduledTask -TaskName $taskName -InputObject $task `
                -User 'HL' -Password '123456' -Force | Out-Null
            Start-ScheduledTask -TaskName $taskName
            $deadline = (Get-Date).AddMinutes(2)
            while (-not (Test-Path -LiteralPath $marker) -and (Get-Date) -lt $deadline) {
                Start-Sleep -Seconds 2
            }
            Start-Sleep -Seconds 1
            $info = Get-ScheduledTaskInfo -TaskName $taskName
            $out.Add("TASK result=$($info.LastTaskResult) state=$((Get-ScheduledTask -TaskName $taskName).State)")
            if (Test-Path -LiteralPath $marker) {
                $out.Add("ELEV_MARKER content=$(Get-Content -LiteralPath $marker -Raw)")
            } else {
                $out.Add("ELEV_MARKER missing")
            }
            Unregister-ScheduledTask -TaskName $taskName -Confirm:$false
        } catch {
            $out.Add("TASK_ERR: $($_.Exception.Message)")
        }
        $out
    }
    foreach ($l in $elev) { $lines.Add($l) }
}
catch {
    $lines.Add("ERROR: $($_.Exception.Message)")
    $lines.Add("STACK: $($_.ScriptStackTrace)")
}
finally {
    if ($sess) {
        Remove-PSSession -Session $sess
    }
}
$lines | Set-Content -LiteralPath $OutFile -Encoding utf8
