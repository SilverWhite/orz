param(
    [Parameter(Mandatory=$true)][string]$Arm,
    [string]$Image = "arm64v8/debian:bookworm-slim"
)

# enforcement-probe 先验墙（无 agent）：三臂各自验证 OS 墙真实生效。
# 用法: .\run_enforcement_probe.ps1 -Arm control|non-root|high-nist
$ErrorActionPreference = "Stop"
$policy = "D:\CLI\_linux_arm_dryrun\policy"

switch ($Arm) {
    "control" {
        docker run --rm --platform linux/arm64/v8 -v "${policy}:/policy" $Image `
            bash -c "bash /policy/enforcement_probe.sh control"
    }
    "non-root" {
        docker run --rm --platform linux/arm64/v8 -v "${policy}:/policy" $Image `
            bash -c "useradd -m -u 1000 agent; mkdir -p /app /usr/local/share/orz-probe; chown agent:agent /app; chmod 0755 /app /usr/local/share/orz-probe; su agent -c 'PROBE_WORKDIR=/app bash /policy/enforcement_probe.sh non-root'"
    }
    "high-nist" {
        docker run --rm --platform linux/arm64/v8 --security-opt no-new-privileges:true `
            --cap-drop ALL --cap-add CHOWN --tmpfs /tmp `
            -v "${policy}:/policy" `
            -v "/usr/local/share/orz-probe:/usr/local/share/orz-probe:ro" $Image `
            bash -c "mkdir -p /home/agent; chmod 0555 /home/agent; PROBE_WORKDIR=/tmp PROBE_HOME=/home/agent PROBE_RO_TARGET=/usr/local/share/orz-probe bash /policy/enforcement_probe.sh high-nist"
    }
    default {
        Write-Error "Unknown arm: $Arm (control|non-root|high-nist)"
    }
}

if ($LASTEXITCODE -ne 0) {
    Write-Error "enforcement-probe $Arm FAILED — 该臂不作数（fail-closed）"
    exit 1
}
Write-Host "enforcement-probe $Arm OK"
