"""Deterministic log generator for the ARM dry-run port of
log-summary-date-ranges (Terminal-Bench 2.1)."""

import datetime
import os
import random


def main() -> None:
    root = "/app/logs"
    os.makedirs(root, exist_ok=True)
    rng = random.Random(20260901)
    # 6 天，每天 20-50 条，时间戳带分钟精度。
    day = datetime.datetime(2026, 8, 20, 0, 0, 0)
    for d in range(6):
        day_path = os.path.join(root, f"{day:%Y-%m-%d}.log")
        n = rng.randint(20, 50)
        with open(day_path, "w", encoding="utf-8") as f:
            for _ in range(n):
                ts = day + datetime.timedelta(minutes=rng.randint(0, 1439))
                level = rng.choice(["INFO", "WARN", "ERROR", "DEBUG"])
                msg = rng.choice(
                    [
                        "request handled",
                        "cache miss",
                        "retry scheduled",
                        "auth ok",
                        "backpressure applied",
                        "connection reused",
                    ]
                )
                f.write(f"{ts:%Y-%m-%d %H:%M:%S} {level} {msg}\n")
        day += datetime.timedelta(days=1)


if __name__ == "__main__":
    main()
