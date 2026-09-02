"""Deterministic seed-log generator for regex-log (ASCII)."""

import os


FILES = {
    "gateway-2026-08-26.log": [
        "2026-08-26 08:01:10 INFO gw-01 request handled",
        "2026-08-26 08:01:11 ERROR gw-01 upstream refused",
        "2026-08-26 08:01:12 WARN gw-01 retry scheduled",
        "2026-08-26 08:01:13 FATAL gw-01 disk full",
        "2026-08-26 08:01:14 INFO gw-01 connection closed",
        "2026-08-26 08:01:15 DEBUG gw-01 cache miss",
    ],
    "worker-2026-08-27.log": [
        "2026-08-27 09:00:00 INFO wk-07 task started",
        "2026-08-27 09:00:01 ERROR wk-07 timeout waiting lock",
        "2026-08-27 09:00:02 INFO wk-07 task resumed",
        "2026-08-27 09:00:03 ERROR wk-07 queue full",
        "2026-08-27 09:00:04 WARN wk-07 backoff 30s",
        "2026-08-27 09:00:05 INFO wk-07 task finished",
        "2026-08-27 09:00:06 DEBUG wk-07 gc run",
    ],
    "cache-2026-08-28.log": [
        "2026-08-28 10:11:12 INFO cache-3 entry evicted",
        "2026-08-28 10:11:13 FATAL cache-3 backend unreachable",
        "2026-08-28 10:11:14 WARN cache-3 retry deferred",
        "2026-08-28 10:11:15 INFO cache-3 compaction done",
        "2026-08-28 10:11:16 DEBUG cache-3 stats written",
    ],
}


def main():
    outdir = os.getcwd()
    total = 0
    for name in sorted(FILES):
        path = os.path.join(outdir, name)
        with open(path, "w", encoding="ascii") as fh:
            for line in FILES[name]:
                fh.write(line + "\n")
                total += 1
    print("SEED_WRITTEN files=%d lines=%d" % (len(FILES), total))


if __name__ == "__main__":
    main()
