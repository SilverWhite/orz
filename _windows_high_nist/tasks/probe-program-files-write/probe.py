"""Friction probe: attempt a Program Files write (P0-0l-④, ASCII)."""

import argparse
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from _tasklib import write_outcome  # noqa: E402


MARKER = "probe-program-files-write-ok"


def _classify(err):
    winerr = getattr(err, "winerror", None)
    if winerr == 5:
        return "denied"
    return "failed"


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("-Arm", required=True)
    ap.add_argument("-Workdir", required=True)
    ap.add_argument("-OutcomePath", required=True)
    args = ap.parse_args()

    task = "probe-program-files-write"
    target_dir = os.path.join(
        os.environ.get("ProgramFiles", r"C:\Program Files"), "OrzS4Probe"
    )
    target = os.path.join(target_dir, "marker.txt")
    error = ""
    attempt = "failed"
    try:
        os.makedirs(target_dir, exist_ok=True)
        with open(target, "w", encoding="utf-8") as fh:
            fh.write(MARKER + "\n")
        with open(target, "r", encoding="utf-8") as fh:
            value = fh.read().strip()
        if value == MARKER:
            attempt = "success"
        else:
            error = "readback mismatch: %r" % (value,)
    except OSError as exc:
        attempt = _classify(exc)
        error = "OSError winerror=%s: %s" % (
            getattr(exc, "winerror", ""),
            exc,
        )
    except Exception as exc:  # noqa: BLE001
        attempt = "failed"
        error = "%s: %s" % (exc.__class__.__name__, exc)
    finally:
        try:
            if os.path.exists(target):
                os.remove(target)
            if os.path.isdir(target_dir):
                os.rmdir(target_dir)
        except OSError:
            pass

    write_outcome(args.OutcomePath, task, args.Arm, attempt, error)
    print("OUTCOME task=%s arm=%s attempt=%s" % (task, args.Arm, attempt))
    return 0


if __name__ == "__main__":
    sys.exit(main())
