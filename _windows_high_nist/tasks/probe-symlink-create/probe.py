"""Friction probe: os.symlink under the arm wall (P0-0l-batch2, ASCII)."""

import argparse
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from _tasklib import write_outcome  # noqa: E402


MARKER = "probe-symlink-create-ok"


def _norm(p):
    p = os.path.abspath(p)
    if p.startswith("\\\\?\\"):
        p = p[4:]
    return os.path.normcase(p)


def _classify(err):
    winerr = getattr(err, "winerror", None)
    # 1314 = privilege not held; 5 = access denied; 1 = function incorrect.
    if winerr in (5, 1, 1314):
        return "denied"
    return "failed"


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("-Arm", required=True)
    ap.add_argument("-Workdir", required=True)
    ap.add_argument("-OutcomePath", required=True)
    args = ap.parse_args()

    task = "probe-symlink-create"
    error = ""
    attempt = "failed"
    target = os.path.join(args.Workdir, "target.txt")
    link = os.path.join(args.Workdir, "link.txt")
    try:
        with open(target, "w", encoding="ascii") as fh:
            fh.write(MARKER + "\n")
        if os.path.islink(link) or os.path.exists(link):
            os.remove(link)
        os.symlink(target, link)
        if not os.path.islink(link):
            raise RuntimeError("symlink not created")
        resolved = os.readlink(link)
        if _norm(resolved) != _norm(target):
            raise RuntimeError("readlink mismatch: %r" % (resolved,))
        with open(link, "r", encoding="ascii") as fh:
            value = fh.read().strip()
        if value != MARKER:
            raise RuntimeError("marker readback mismatch via link")
        attempt = "success"
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
        for p in (link, target):
            try:
                if os.path.islink(p) or os.path.isfile(p):
                    os.remove(p)
            except OSError:
                pass

    write_outcome(args.OutcomePath, task, args.Arm, attempt, error)
    print("OUTCOME task=%s arm=%s attempt=%s" % (task, args.Arm, attempt))
    return 0


if __name__ == "__main__":
    sys.exit(main())
