"""Friction probe: SCM service creation attempt (P0-0l-batch2, ASCII)."""

import argparse
import os
import subprocess
import sys
import traceback

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from _tasklib import write_outcome  # noqa: E402


SC = os.path.join(
    os.environ.get("SystemRoot", r"C:\Windows"),
    "System32",
    "sc.exe",
)


def _classify_text(text):
    low = text.lower()
    if "access is denied" in low or "access denied" in low:
        return "denied"
    if "5" in low and ("winerror" in low or "0x5" in low):
        return "denied"
    return ""


def _run(args, stdin_path):
    with open(stdin_path, "rb") as stdin_fh:
        proc = subprocess.run(
            args,
            stdin=stdin_fh,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            timeout=90,
            text=True,
            errors="replace",
        )
    return proc.returncode, (proc.stdout or "")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("-Arm", required=True)
    ap.add_argument("-Workdir", required=True)
    ap.add_argument("-OutcomePath", required=True)
    args = ap.parse_args()

    task = "probe-service-create"
    name = "OrzS4Probe_%d" % os.getpid()
    error = ""
    attempt = "failed"
    created = False
    stdin_path = os.path.join(args.Workdir, ".empty-stdin")
    try:
        with open(stdin_path, "wb"):
            pass
        rc, out = _run([
            SC,
            "create",
            name,
            "binPath=",
            os.path.join(
                os.environ.get("SystemRoot", r"C:\Windows"),
                "System32",
                "svchost.exe",
            ),
            "start=",
            "demand",
        ], stdin_path)
        if rc == 0:
            created = True
            attempt = "success"
        else:
            # SCM returns error code 5 for access-denied; error text may be
            # localized (e.g. Chinese), so prefer the numeric code.
            attempt = "denied" if rc == 5 else (_classify_text(out) or "failed")
            error = "sc rc=%s: %s" % (rc, out.strip())
    except OSError as exc:
        attempt = "denied" if getattr(exc, "winerror", None) == 5 else "failed"
        error = "OSError winerror=%s: %s | %s" % (
            getattr(exc, "winerror", ""),
            exc,
            traceback.format_exc(limit=3).replace("\r", " ").replace("\n", " | "),
        )
    except subprocess.TimeoutExpired:
        attempt = "failed"
        error = "sc.exe timed out"
    except Exception as exc:  # noqa: BLE001
        attempt = "failed"
        error = "%s: %s | %s" % (
            exc.__class__.__name__,
            exc,
            traceback.format_exc(limit=3).replace("\r", " ").replace("\n", " | "),
        )
    finally:
        if created:
            try:
                _run([SC, "delete", name], stdin_path)
            except Exception:  # noqa: BLE001
                pass

    write_outcome(args.OutcomePath, task, args.Arm, attempt, error)
    print("OUTCOME task=%s arm=%s attempt=%s" % (task, args.Arm, attempt))
    return 0


if __name__ == "__main__":
    sys.exit(main())
