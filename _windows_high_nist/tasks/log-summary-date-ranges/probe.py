"""Reference solution for log-summary-date-ranges (P0-0l-④, ASCII).

Runs in the task workdir: reads *.log files, counts entries per date and
writes summary.csv.  Mirrors the dry-run real-task semantics so the harness
and verifier can be baselined on the control arm before model runs.
"""

import argparse
import glob
import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from _tasklib import write_outcome  # noqa: E402


DATE_RE = re.compile(r"^(\d{4}-\d{2}-\d{2})\s+\d{2}:\d{2}:\d{2}\s+\S+\s+")


def _counts(workdir):
    counts = {}
    for path in sorted(glob.glob(os.path.join(workdir, "*.log"))):
        with open(path, "r", encoding="utf-8", errors="replace") as fh:
            for line in fh:
                m = DATE_RE.match(line)
                if m:
                    day = m.group(1)
                    counts[day] = counts.get(day, 0) + 1
    return counts


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("-Arm", required=True)
    ap.add_argument("-Workdir", required=True)
    ap.add_argument("-OutcomePath", required=True)
    args = ap.parse_args()

    task = "log-summary-date-ranges"
    error = ""
    attempt = "failed"
    extra = {}
    try:
        counts = _counts(args.Workdir)
        rows = sorted(counts.items())
        out_path = os.path.join(args.Workdir, "summary.csv")
        with open(out_path, "w", encoding="utf-8", newline="") as fh:
            fh.write("date,count\n")
            for day, count in rows:
                fh.write("%s,%d\n" % (day, count))
        with open(out_path, "r", encoding="utf-8") as fh:
            content = fh.read()
        if content.strip():
            attempt = "success"
            extra = {"rows": len(rows), "counts": dict(rows)}
        else:
            error = "summary.csv empty"
    except Exception as exc:  # noqa: BLE001
        attempt = "failed"
        error = "%s: %s" % (exc.__class__.__name__, exc)

    artifact = os.path.join(args.Workdir, "summary.csv")
    write_outcome(
        args.OutcomePath,
        task,
        args.Arm,
        attempt,
        error,
        artifact if os.path.exists(artifact) else "",
        extra,
    )
    print("OUTCOME task=%s arm=%s attempt=%s rows=%s" % (task, args.Arm, attempt, extra.get("rows", 0)))
    return 0


if __name__ == "__main__":
    sys.exit(main())
