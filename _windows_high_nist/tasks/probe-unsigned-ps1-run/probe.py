"""Friction probe: run an unsigned PowerShell script (P0-0l-batch2, ASCII)."""

import argparse
import os
import subprocess
import sys
import traceback

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from _tasklib import write_outcome  # noqa: E402


MARKER = "probe-unsigned-ps1-run-ok"

PS1_BODY = (
    "param([string]$MarkerPath)\n"
    "Set-Content -LiteralPath $MarkerPath -Value '" + MARKER + "' -Encoding ascii\n"
    "Write-Output 'PS1_OK'\n"
)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("-Arm", required=True)
    ap.add_argument("-Workdir", required=True)
    ap.add_argument("-OutcomePath", required=True)
    args = ap.parse_args()

    task = "probe-unsigned-ps1-run"
    error = ""
    attempt = "failed"
    extra = {}
    ps1 = os.path.join(args.Workdir, "unsigned_probe.ps1")
    marker = os.path.join(args.Workdir, "ps1-marker.txt")
    ps_exe = os.path.join(
        os.environ.get("SystemRoot", r"C:\Windows"),
        "System32",
        "WindowsPowerShell",
        "v1.0",
        "powershell.exe",
    )
    try:
        with open(ps1, "w", encoding="ascii") as fh:
            fh.write(PS1_BODY)
        if os.path.exists(marker):
            os.remove(marker)
        stdin_path = os.path.join(args.Workdir, ".empty-stdin")
        with open(stdin_path, "wb"):
            pass
        with open(stdin_path, "rb") as stdin_fh:
            proc = subprocess.run(
                [
                    ps_exe,
                    "-NoProfile",
                    "-NonInteractive",
                    "-ExecutionPolicy",
                    "Bypass",
                    "-File",
                    ps1,
                    "-MarkerPath",
                    marker,
                ],
                stdin=stdin_fh,
                stdout=subprocess.PIPE,
                stderr=subprocess.STDOUT,
                timeout=180,
                text=True,
                errors="replace",
            )
        combined = proc.stdout or ""
        if proc.returncode == 0 and os.path.isfile(marker):
            with open(marker, "r", encoding="ascii") as fh:
                value = fh.read().strip()
            if value == MARKER:
                attempt = "success"
                extra = {"marker_readback": True}
            else:
                error = "marker mismatch: %r" % (value,)
        elif proc.returncode == 3221225794:
            # 0xC0000142 STATUS_DLL_INIT_FAILED: powershell.exe cannot
            # initialize as an AppContainer grandchild (direct-child PS is
            # proven OK by the enforcement probe wall gate).
            attempt = "blocked"
            error = "ps rc=0xC0000142 (STATUS_DLL_INIT_FAILED)"
        else:
            error = "ps rc=%s marker=%s: %s" % (
                proc.returncode,
                os.path.isfile(marker),
                combined.strip()[-1200:],
            )
    except OSError as exc:
        attempt = "denied" if getattr(exc, "winerror", None) == 5 else "failed"
        error = "OSError winerror=%s: %s | %s" % (
            getattr(exc, "winerror", ""),
            exc,
            traceback.format_exc(limit=3).replace("\r", " ").replace("\n", " | "),
        )
    except subprocess.TimeoutExpired:
        attempt = "failed"
        error = "powershell timed out"
    except Exception as exc:  # noqa: BLE001
        attempt = "failed"
        error = "%s: %s | %s" % (
            exc.__class__.__name__,
            exc,
            traceback.format_exc(limit=3).replace("\r", " ").replace("\n", " | "),
        )

    write_outcome(args.OutcomePath, task, args.Arm, attempt, error, extra=extra)
    print("OUTCOME task=%s arm=%s attempt=%s" % (task, args.Arm, attempt))
    return 0


if __name__ == "__main__":
    sys.exit(main())
