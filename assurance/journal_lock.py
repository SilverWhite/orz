from __future__ import annotations

from contextlib import contextmanager
import math
import os
from pathlib import Path
import time
from typing import BinaryIO, Iterator

from .errors import AssuranceError


def journal_lock_path(journal_path: Path) -> Path:
    return Path(f"{journal_path}.lock")


def _ensure_lock_byte(handle: BinaryIO) -> None:
    handle.seek(0, os.SEEK_END)
    if handle.tell() == 0:
        handle.write(b"\0")
        handle.flush()
        os.fsync(handle.fileno())
    handle.seek(0)


def _try_lock(handle: BinaryIO) -> bool:
    handle.seek(0)
    if os.name == "nt":
        import msvcrt

        try:
            msvcrt.locking(handle.fileno(), msvcrt.LK_NBLCK, 1)
        except OSError:
            return False
        return True
    if os.name == "posix":
        import fcntl

        try:
            fcntl.flock(handle.fileno(), fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            return False
        return True
    raise AssuranceError(f"unsupported journal-lock platform: {os.name}")


def _unlock(handle: BinaryIO) -> None:
    handle.seek(0)
    if os.name == "nt":
        import msvcrt

        msvcrt.locking(handle.fileno(), msvcrt.LK_UNLCK, 1)
        return
    if os.name == "posix":
        import fcntl

        fcntl.flock(handle.fileno(), fcntl.LOCK_UN)
        return
    raise AssuranceError(f"unsupported journal-lock platform: {os.name}")


@contextmanager
def exclusive_journal_lock(
    journal_path: Path,
    *,
    timeout_seconds: float = 5.0,
    poll_interval_seconds: float = 0.02,
) -> Iterator[Path]:
    if (
        not math.isfinite(timeout_seconds)
        or timeout_seconds < 0
        or not math.isfinite(poll_interval_seconds)
        or poll_interval_seconds <= 0
    ):
        raise AssuranceError("journal lock timeout and poll interval must be finite")

    lock_path = journal_lock_path(journal_path)
    lock_path.parent.mkdir(parents=True, exist_ok=True)
    deadline = time.monotonic() + timeout_seconds
    with lock_path.open("a+b") as handle:
        _ensure_lock_byte(handle)
        while not _try_lock(handle):
            if time.monotonic() >= deadline:
                raise AssuranceError(
                    f"timed out acquiring journal lock after {timeout_seconds:.3f}s"
                )
            time.sleep(min(poll_interval_seconds, max(0.0, deadline - time.monotonic())))
        try:
            yield lock_path
        finally:
            _unlock(handle)
