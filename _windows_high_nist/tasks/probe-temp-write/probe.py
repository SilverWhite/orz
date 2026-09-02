"""Friction probe: %TEMP% write via stdlib tempfile (P0-0l-batch2, ASCII)."""

import argparse
import os
import sys
import tempfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from _tasklib import write_outcome  # noqa: E402


MARKER = "probe-temp-write-ok"


def _classify(err):
    winerr = getattr(err, "winerror", None)
    if winerr == 5:
        return "denied"
    return "failed"


def _under_workdir(path, workdir):
    try:
        p = os.path.normcase(os.path.realpath(path))
        w = os.path.normcase(os.path.realpath(workdir))
        return p == w or p.startswith(w + os.sep)
    except OSError:
        return False


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("-Arm", required=True)
    ap.add_argument("-Workdir", required=True)
    ap.add_argument("-OutcomePath", required=True)
    args = ap.parse_args()

    task = "probe-temp-write"
    error = ""
    attempt = "failed"
    extra = {}
    target = ""
    try:
        tmp = tempfile.gettempdir()
        if not tmp:
            raise OSError("empty tempfile.gettempdir()")
        target = os.path.join(tmp, "OrzS4ProbeTmp_%d.txt" % os.getpid())
        with open(target, "w", encoding="ascii") as fh:
            fh.write(MARKER + "\n")
        with open(target, "r", encoding="ascii") as fh:
            value = fh.read().strip()
        if value != MARKER:
            raise RuntimeError("readback mismatch")
        attempt = "success"
        extra = {
            "temp": tmp,
            "temp_under_workspace": _under_workdir(tmp, args.Workdir),
        }
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
            if target and os.path.exists(target):
                os.remove(target)
        except OSError:
            pass

    write_outcome(args.OutcomePath, task, args.Arm, attempt, error, extra=extra)
    print("OUTCOME task=%s arm=%s attempt=%s temp_under=%s" % (
        task,
        args.Arm,
        attempt,
        extra.get("temp_under_workspace", False),
    ))
    return 0


if __name__ == "__main__":
    sys.exit(main())
