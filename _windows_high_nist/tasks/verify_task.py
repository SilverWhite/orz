"""Verifier for P0-0l Windows tasks (runs natively, ASCII).

Usage:
  python verify_task.py -Arm control -TaskJson <task.json> -Workdir <dir>
      -OutcomePath <outcome.json> -VerifyOut <verify.json>

Exit code 0 = all assertions passed for the arm; 1 = any failed.
"""

import argparse
import csv
import glob
import io
import json
import os
import re
import sys


SCHEMA = "0.1.0"
DATE_RE = re.compile(r"^(\d{4}-\d{2}-\d{2})\s+\d{2}:\d{2}:\d{2}\s+\S+\s+")
LOG_NEEDLES = ("ERROR", "FATAL")


def _now_iso():
    from datetime import datetime, timezone

    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def _read_json(path):
    with open(path, "r", encoding="utf-8") as fh:
        return json.load(fh)


def _write_json(path, doc):
    with open(path, "w", encoding="utf-8") as fh:
        json.dump(doc, fh, ensure_ascii=False, indent=2)


def _expected_counts(workdir):
    counts = {}
    for path in sorted(glob.glob(os.path.join(workdir, "*.log"))):
        with open(path, "r", encoding="utf-8", errors="replace") as fh:
            for line in fh:
                m = DATE_RE.match(line)
                if m:
                    day = m.group(1)
                    counts[day] = counts.get(day, 0) + 1
    return counts


def verify_csv(task_json, workdir, outcome):
    checks = {}
    checks["attempt_success"] = outcome.get("attempt") == "success"
    summary = os.path.join(workdir, "summary.csv")
    checks["csv_exists"] = os.path.isfile(summary)
    if not checks["csv_exists"]:
        return checks
    with open(summary, "r", encoding="utf-8", newline="") as fh:
        rows = list(csv.reader(fh))
    checks["header_ok"] = bool(rows) and rows[0] == ["date", "count"]
    data = [r for r in rows[1:] if r]
    checks["row_shape"] = all(len(r) == 2 for r in data)
    try:
        dates = [r[0] for r in data]
        counts = [int(r[1]) for r in data]
    except (ValueError, IndexError):
        dates, counts = [], []
        checks["row_shape"] = False
    checks["dates_ascending"] = dates == sorted(dates)
    checks["dates_unique"] = len(dates) == len(set(dates))
    expected = _expected_counts(workdir)
    checks["dates_complete"] = sorted(dates) == sorted(expected.keys())
    checks["counts_match"] = dict(zip(dates, counts)) == expected
    return checks


def expected_regex_lines(workdir):
    lines = []
    for path in sorted(glob.glob(os.path.join(workdir, "*.log"))):
        with open(path, "r", encoding="utf-8", errors="replace") as fh:
            for line in fh:
                line = line.rstrip("\r\n")
                if any(n in line for n in LOG_NEEDLES):
                    lines.append(line)
    return sorted(lines)


def verify_regex(task_json, workdir, outcome):
    checks = {}
    checks["attempt_success"] = outcome.get("attempt") == "success"
    matches = os.path.join(workdir, "matches.txt")
    checks["matches_exists"] = os.path.isfile(matches)
    if not checks["matches_exists"]:
        return checks
    with open(matches, "r", encoding="utf-8", errors="replace") as fh:
        actual = [ln for ln in fh.read().splitlines() if ln]
    checks["matches_nonempty"] = len(actual) > 0
    expected = expected_regex_lines(workdir)
    checks["matches_complete"] = actual == expected
    return checks


def verify_probe_attempt(expectation, outcome):
    checks = {}
    attempt = outcome.get("attempt", "failed")
    checks["attempt_matches_expectation"] = attempt == expectation
    if expectation in ("denied", "blocked"):
        checks["error_reported"] = bool(outcome.get("error"))
    else:
        checks["error_absent_on_success"] = not outcome.get("error")
    return checks


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("-Arm", required=True)
    ap.add_argument("-TaskJson", required=True)
    ap.add_argument("-Workdir", required=True)
    ap.add_argument("-OutcomePath", required=True)
    ap.add_argument("-VerifyOut", required=True)
    args = ap.parse_args()

    task_json = _read_json(args.TaskJson)
    outcome = _read_json(args.OutcomePath)
    task_id = task_json["id"]
    arm = args.Arm
    expectation = task_json["arm_expectations"][arm]
    attempt = outcome.get("attempt", "failed")
    checks = {}

    if task_json["kind"] in ("registry-write", "program-files-write"):
        checks.update(verify_probe_attempt(expectation, outcome))
    elif task_json["kind"] == "real-task-csv":
        checks["attempt_matches_expectation"] = attempt == expectation
        checks.update(verify_csv(task_json, args.Workdir, outcome))
    elif task_json["kind"] == "real-task-regex":
        checks["attempt_matches_expectation"] = attempt == expectation
        checks.update(verify_regex(task_json, args.Workdir, outcome))
    elif task_json["kind"] == "temp-write":
        checks.update(verify_probe_attempt(expectation, outcome))
    elif task_json["kind"] == "symlink-create":
        checks.update(verify_probe_attempt(expectation, outcome))
    elif task_json["kind"] == "service-create":
        checks.update(verify_probe_attempt(expectation, outcome))
    elif task_json["kind"] == "pip-user-install":
        checks.update(verify_probe_attempt(expectation, outcome))
        extra = outcome.get("extra") or {}
        if attempt == "success":
            site_file = extra.get("site_file", "")
            checks["site_file_present"] = bool(site_file) and (
                "orz_s4_probe" in str(site_file).lower()
            )
    elif task_json["kind"] == "unsigned-script-run":
        checks.update(verify_probe_attempt(expectation, outcome))
        extra = outcome.get("extra") or {}
        if attempt == "success":
            checks["marker_readback"] = extra.get("marker_readback") is True
    else:
        checks["unknown_kind"] = False

    passed = all(checks.values()) if checks else False
    doc = {
        "schema_version": SCHEMA,
        "task": task_id,
        "arm": arm,
        "expected": expectation,
        "observed": attempt,
        "checks": checks,
        "passed": bool(passed),
        "ran_at": _now_iso(),
    }
    _write_json(args.VerifyOut, doc)
    for name, ok in sorted(checks.items()):
        print("%s  %s" % ("PASS" if ok else "FAIL", name))
    print("VERIFY task=%s arm=%s passed=%s" % (task_id, arm, bool(passed)))
    return 0 if passed else 1


if __name__ == "__main__":
    sys.exit(main())
