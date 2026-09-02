"""Reference solution for regex-log (Windows port, P0-0l-batch2, ASCII)."""

import argparse
import glob
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from _tasklib import write_outcome  # noqa: E402


NEEDLES = ("ERROR", "FATAL")


def _matching_lines(workdir):
    lines = []
    for path in sorted(glob.glob(os.path.join(workdir, "*.log"))):
        with open(path, "r", encoding="ascii", errors="replace") as fh:
            for line in fh:
                line = line.rstrip("\r\n")
                if any(n in line for n in NEEDLES):
                    lines.append(line)
    return sorted(lines)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("-Arm", required=True)
    ap.add_argument("-Workdir", required=True)
    ap.add_argument("-OutcomePath", required=True)
    args = ap.parse_args()

    task = "regex-log"
    error = ""
    attempt = "failed"
    extra = {}
    out_path = os.path.join(args.Workdir, "matches.txt")
    try:
        lines = _matching_lines(args.Workdir)
        if not lines:
            raise RuntimeError("no matching lines found")
        with open(out_path, "w", encoding="ascii") as fh:
            fh.write("\n".join(lines) + "\n")
        with open(out_path, "r", encoding="ascii") as fh:
            content = fh.read()
        readback = [ln for ln in content.splitlines() if ln]
        if readback != lines:
            raise RuntimeError("readback mismatch")
        attempt = "success"
        extra = {"matched_lines": len(lines)}
    except Exception as exc:  # noqa: BLE001
        attempt = "failed"
        error = "%s: %s" % (exc.__class__.__name__, exc)

    artifact = out_path if os.path.exists(out_path) else ""
    write_outcome(
        args.OutcomePath,
        task,
        args.Arm,
        attempt,
        error,
        artifact,
        extra,
    )
    print("OUTCOME task=%s arm=%s attempt=%s matched=%s" % (
        task,
        args.Arm,
        attempt,
        extra.get("matched_lines", 0),
    ))
    return 0


if __name__ == "__main__":
    sys.exit(main())
