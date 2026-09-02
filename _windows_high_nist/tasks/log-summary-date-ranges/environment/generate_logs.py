"""Deterministic seed-log generator for log-summary-date-ranges (ASCII)."""

import os


DAYS = [
    ("2026-08-20", 3),
    ("2026-08-21", 5),
    ("2026-08-22", 1),
    ("2026-08-23", 4),
    ("2026-08-24", 2),
    ("2026-08-25", 6),
]

MESSAGES = [
    "INFO startup complete",
    "WARN retryable timeout",
    "ERROR upstream refused",
    "INFO request handled",
    "DEBUG cache miss",
]


def main():
    outdir = os.getcwd()
    for day, count in DAYS:
        path = os.path.join(outdir, day + ".log")
        with open(path, "w", encoding="utf-8") as fh:
            for i in range(count):
                hh = 8 + (i % 10)
                mm = (i * 7) % 60
                ss = (i * 13) % 60
                msg = MESSAGES[i % len(MESSAGES)]
                fh.write(
                    "%s %02d:%02d:%02d %s\n" % (day, hh, mm, ss, msg)
                )
    print("SEED_WRITTEN files=%d" % len(DAYS))


if __name__ == "__main__":
    main()
