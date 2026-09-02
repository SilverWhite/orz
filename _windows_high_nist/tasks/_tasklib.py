"""Shared helpers for P0-0l-④ Windows task probes / verifiers (ASCII)."""

import datetime
import json
import os


SCHEMA = "0.1.0"


def now_iso():
    return datetime.datetime.now(datetime.timezone.utc).strftime(
        "%Y-%m-%dT%H:%M:%SZ"
    )


def write_json(path, doc):
    tmp = path + ".tmp"
    with open(tmp, "w", encoding="utf-8") as fh:
        json.dump(doc, fh, ensure_ascii=False, indent=2)
    os.replace(tmp, path)


def read_json(path):
    with open(path, "r", encoding="utf-8") as fh:
        return json.load(fh)


def write_outcome(path, task, arm, attempt, error="", artifact="", extra=None):
    write_json(
        path,
        {
            "schema_version": SCHEMA,
            "task": task,
            "arm": arm,
            "attempt": attempt,
            "error": error,
            "artifact": artifact,
            "extra": extra or {},
            "ran_at": now_iso(),
        },
    )
