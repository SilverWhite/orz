from __future__ import annotations

import json
import math
import os
import time
import uuid
from contextlib import contextmanager
from pathlib import Path
from typing import Any, BinaryIO, Iterator

from .contracts import validate_contract
from .errors import AssuranceError
from .utils import canonical_bytes, load_json, sha256_bytes, utc_now


# ── advisory file lock (same pattern as prototype/fep_agent_proto/journal_lock.py) ──


def _archive_lock_path(journal_path: Path) -> Path:
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
    raise OSError(f"unsupported lock platform: {os.name}")


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
    raise OSError(f"unsupported lock platform: {os.name}")


@contextmanager
def exclusive_archive_lock(
    journal_path: Path,
    *,
    timeout_seconds: float = 5.0,
    poll_interval_seconds: float = 0.02,
) -> Iterator[Path]:
    """Acquire an exclusive advisory lock on an archive journal file."""
    if (
        not math.isfinite(timeout_seconds)
        or timeout_seconds < 0
        or not math.isfinite(poll_interval_seconds)
        or poll_interval_seconds <= 0
    ):
        raise AssuranceError(
            "archive lock timeout and poll interval must be finite and valid"
        )
    lock_path = _archive_lock_path(journal_path)
    lock_path.parent.mkdir(parents=True, exist_ok=True)
    deadline = time.monotonic() + timeout_seconds
    with lock_path.open("a+b") as handle:
        _ensure_lock_byte(handle)
        while not _try_lock(handle):
            if time.monotonic() >= deadline:
                raise AssuranceError(
                    f"timed out acquiring archive lock after {timeout_seconds:.3f}s"
                )
            time.sleep(
                min(poll_interval_seconds, max(0.0, deadline - time.monotonic()))
            )
        try:
            yield lock_path
        finally:
            _unlock(handle)


# ── event hashing ──


def _event_hash(event: dict[str, Any]) -> str:
    """SHA-256 of canonical JSON minus event_sha256 key."""
    body = {k: v for k, v in event.items() if k != "event_sha256"}
    return sha256_bytes(canonical_bytes(body))


def _canonical_json_line(event: dict[str, Any]) -> bytes:
    """Compact JSON line with sorted keys and trailing newline."""
    return (
        json.dumps(event, sort_keys=True, ensure_ascii=False, separators=(",", ":"))
        .encode("utf-8")
        + b"\n"
    )


def _validate_event(event: dict[str, Any], *, label: str = "") -> None:
    validate_contract(
        event,
        "archive-operation-journal-event-v0.1.schema.json",
        label=label or "archive journal event",
    )


# ── journal replay ──


def replay_archive_journal(journal_path: Path) -> dict[str, Any]:
    """Read-only replay of an archive operation journal.

    Returns a dict with keys:
      - valid: bool
      - events: list of parsed events
      - errors: list of error strings
      - deleted_files: set of (category, relative_path) already deleted
      - last_sequence: int (next available sequence number)
      - last_event_sha256: str | None
      - terminal_event: str | None (None = interrupted / no terminal)
      - event_count: int
    """
    errors: list[str] = []
    events: list[dict[str, Any]] = []
    deleted_files: set[tuple[str, str]] = set()
    last_sequence = -1
    last_event_sha256: str | None = None
    terminal_event: str | None = None
    prev_hash: str | None = None

    if not journal_path.exists():
        return {
            "valid": True,
            "events": [],
            "errors": [],
            "deleted_files": deleted_files,
            "last_sequence": 0,
            "last_event_sha256": None,
            "terminal_event": None,
            "event_count": 0,
        }

    try:
        raw = journal_path.read_bytes()
    except OSError as exc:
        return {
            "valid": False,
            "events": [],
            "errors": [f"cannot read journal: {exc}"],
            "deleted_files": deleted_files,
            "last_sequence": 0,
            "last_event_sha256": None,
            "terminal_event": None,
            "event_count": 0,
        }

    if not raw:
        return {
            "valid": True,
            "events": [],
            "errors": [],
            "deleted_files": deleted_files,
            "last_sequence": 0,
            "last_event_sha256": None,
            "terminal_event": None,
            "event_count": 0,
        }

    lines = raw.split(b"\n")
    # Last empty line after trailing newline is not a record
    if lines and lines[-1] == b"":
        lines = lines[:-1]

    for idx, line in enumerate(lines):
        if not line.strip():
            errors.append(f"empty line at journal offset {idx}")
            break
        try:
            event = json.loads(line.decode("utf-8"))
        except (json.JSONDecodeError, UnicodeDecodeError) as exc:
            errors.append(f"invalid JSON at line {idx}: {exc}")
            break

        try:
            _validate_event(event, label=f"archive journal event {idx}")
        except AssuranceError as exc:
            errors.append(str(exc))
            break

        # Stop replay at terminal events — archive is already completed
        if terminal_event is not None:
            errors.append(
                f"event after terminal {terminal_event} at sequence {event['sequence']}"
            )
            break

        # Validate hash chain
        if event["sequence"] != last_sequence + 1:
            errors.append(
                f"sequence gap: expected {last_sequence + 1}, got {event['sequence']}"
            )
            break

        if event["sequence"] == 0:
            if event["previous_event_sha256"] is not None:
                errors.append("first event must have null previous_event_sha256")
                break
        else:
            if event["previous_event_sha256"] != last_event_sha256:
                errors.append(
                    f"hash chain break at sequence {event['sequence']}: "
                    f"expected {last_event_sha256}, got {event['previous_event_sha256']}"
                )
                break

        computed_hash = _event_hash(event)
        if computed_hash != event["event_sha256"]:
            errors.append(
                f"event hash mismatch at sequence {event['sequence']}: "
                f"expected {event['event_sha256']}, computed {computed_hash}"
            )
            break

        if event["payload_sha256"] != sha256_bytes(canonical_bytes(event["payload"])):
            errors.append(f"payload digest mismatch at sequence {event['sequence']}")
            break

        events.append(event)
        last_sequence = event["sequence"]
        last_event_sha256 = event["event_sha256"]

        # Track deleted files
        if event["event_type"] == "file_deleted":
            payload = event["payload"]
            deleted_files.add((payload["category"], payload["relative_path"]))

        if event["event_type"] in {"archive_completed", "archive_failed"}:
            terminal_event = event["event_type"]

    return {
        "valid": not errors,
        "events": events,
        "errors": errors,
        "deleted_files": deleted_files,
        "last_sequence": last_sequence + 1,
        "last_event_sha256": last_event_sha256,
        "terminal_event": terminal_event,
        "event_count": len(events),
    }


# ── journal recovery ──


def _classify_journal_tail(raw: bytes) -> tuple[str, int | None]:
    """Classify a raw journal byte string.

    Returns (classification, valid_prefix_length).
    Classifications: "clean", "torn_tail", "missing_newline", "empty", "corrupt"
    """
    if not raw:
        return "empty", 0

    # Find last complete newline-terminated line
    last_nl = raw.rfind(b"\n")
    if last_nl == -1:
        return "torn_tail", 0

    # Check if there are non-whitespace bytes after the last newline
    trailing = raw[last_nl + 1 :]
    if trailing.strip():
        return "torn_tail", last_nl + 1  # valid prefix ends after the last newline

    # All lines are newline-terminated — check each line is valid JSON
    # But doing full validation here would duplicate replay. Just check JSON parse.
    lines = raw.split(b"\n")
    if lines and lines[-1] == b"":
        lines = lines[:-1]
    for idx, line in enumerate(lines):
        if not line.strip():
            return "corrupt", None
        try:
            json.loads(line.decode("utf-8"))
        except (json.JSONDecodeError, UnicodeDecodeError):
            return "corrupt", None

    return "clean", None


def inspect_archive_journal(
    journal_path: Path,
    *,
    lock_timeout_seconds: float = 5.0,
) -> dict[str, Any]:
    """Read-only inspection of archive journal for recovery classification.

    Returns a dict with keys:
      - valid: bool
      - classification: str ("clean", "torn_tail", "missing_newline", "empty", "corrupt")
      - prefix_length: int | None (valid prefix bytes, None if corrupt/clean/empty)
      - tail_bytes: int (bytes after prefix, 0 if clean)
      - replay: dict (result of replay on the valid prefix)
    """
    if not journal_path.exists():
        return {
            "valid": True,
            "classification": "empty",
            "prefix_length": 0,
            "tail_bytes": 0,
            "replay": replay_archive_journal(journal_path),
        }

    try:
        raw = journal_path.read_bytes()
    except OSError as exc:
        return {
            "valid": False,
            "classification": "corrupt",
            "prefix_length": None,
            "tail_bytes": 0,
            "replay": {"valid": False, "errors": [str(exc)]},
        }

    classification, prefix_length = _classify_journal_tail(raw)

    if classification == "clean":
        replay_result = replay_archive_journal(journal_path)
        return {
            "valid": replay_result["valid"],
            "classification": "clean" if replay_result["valid"] else "corrupt",
            "prefix_length": None,
            "tail_bytes": 0,
            "replay": replay_result,
        }

    if classification == "empty":
        return {
            "valid": True,
            "classification": "empty",
            "prefix_length": 0,
            "tail_bytes": 0,
            "replay": replay_archive_journal(journal_path),
        }

    if classification == "corrupt":
        return {
            "valid": False,
            "classification": "corrupt",
            "prefix_length": None,
            "tail_bytes": len(raw),
            "replay": {"valid": False, "errors": ["journal is corrupt"]},
        }

    # torn_tail / missing_newline
    if prefix_length is None:
        prefix_length = 0
    tail_bytes = len(raw) - prefix_length

    # Replay the valid prefix
    prefix_valid = False
    replay_result: dict[str, Any] = {"valid": False, "errors": ["could not replay prefix"]}
    if prefix_length > 0:
        try:
            prefix_data = raw[:prefix_length]
            # Ensure trailing newline for replay
            if prefix_data and prefix_data[-1:] != b"\n":
                prefix_data += b"\n"
            import tempfile

            with tempfile.NamedTemporaryFile(
                suffix=".jsonl", delete=False
            ) as tmp:
                tmp.write(prefix_data)
                tmp.flush()
                replay_result = replay_archive_journal(Path(tmp.name))
                prefix_valid = replay_result["valid"]
            os.unlink(tmp.name)
        except OSError:
            replay_result = {"valid": False, "errors": ["prefix replay failed"]}

    return {
        "valid": prefix_valid,
        "classification": classification,
        "prefix_length": prefix_length,
        "tail_bytes": tail_bytes,
        "replay": replay_result,
    }


def recover_archive_journal(
    journal_path: Path,
    *,
    receipt_path: Path | None = None,
    quarantine_path: Path | None = None,
    lock_timeout_seconds: float = 5.0,
) -> dict[str, Any]:
    """Recover a torn archive journal by truncating to the last valid newline.

    Only operates when inspection reports "torn_tail" or "missing_newline".
    Requires an exclusive lock on the journal.

    Returns a recovery receipt dict.
    """
    if not journal_path.exists():
        return {
            "valid": False,
            "applied": False,
            "classification": "empty",
            "errors": ["journal does not exist"],
        }

    # Inspect without lock first
    inspection = inspect_archive_journal(
        journal_path, lock_timeout_seconds=lock_timeout_seconds
    )
    if inspection["classification"] not in {"torn_tail", "missing_newline"}:
        return {
            "valid": inspection["valid"],
            "applied": False,
            "classification": inspection["classification"],
            "errors": [
                f"journal classification '{inspection['classification']}' is not recoverable"
            ],
        }

    # Acquire lock and repair
    with exclusive_archive_lock(
        journal_path, timeout_seconds=lock_timeout_seconds
    ):
        raw = journal_path.read_bytes()
        classification, prefix_length = _classify_journal_tail(raw)
        if classification not in {"torn_tail", "missing_newline"}:
            return {
                "valid": False,
                "applied": False,
                "classification": classification,
                "errors": ["journal classification changed under lock"],
            }

        if prefix_length is None or prefix_length == 0:
            return {
                "valid": False,
                "applied": False,
                "classification": classification,
                "errors": ["no recoverable prefix"],
            }

        discarded = raw[prefix_length:]
        prefix = raw[:prefix_length]

        # Quarantine discarded bytes
        if quarantine_path is not None and discarded:
            quarantine_path.parent.mkdir(parents=True, exist_ok=True)
            quarantine_path.write_bytes(discarded)

        # Truncate journal to valid prefix, ensure trailing newline
        if prefix and prefix[-1:] != b"\n":
            prefix += b"\n"
        journal_path.write_bytes(prefix)

        # Write recovery event
        replay = replay_archive_journal(journal_path)
        recovery_event = {
            "schema_version": "0.1.0-draft",
            "archive_id": replay["events"][0]["archive_id"]
            if replay["events"]
            else "ARC-RECOVERY",
            "sequence": replay["last_sequence"],
            "timestamp": utc_now(),
            "conversation_id": replay["events"][0]["conversation_id"]
            if replay["events"]
            else "CONV-UNKNOWN",
            "event_type": "archive_recovered",
            "previous_event_sha256": replay["last_event_sha256"],
            "payload": {
                "recovery_id": f"AREC-{uuid.uuid4().hex.upper()}",
                "original_tail_sha256": sha256_bytes(discarded) if discarded else "",
                "discarded_size_bytes": len(discarded),
                "prefix_event_count": replay["event_count"],
            },
            "payload_sha256": "",
            "event_sha256": "",
        }
        recovery_event["payload_sha256"] = sha256_bytes(
            canonical_bytes(recovery_event["payload"])
        )
        recovery_event["event_sha256"] = _event_hash(recovery_event)
        _validate_event(recovery_event, label="archive recovery event")

        with journal_path.open("ab") as handle:
            handle.write(_canonical_json_line(recovery_event))
            handle.flush()
            os.fsync(handle.fileno())

    receipt = {
        "schema_version": "0.1.0-draft",
        "receipt_kind": "archive_journal_recovery_receipt",
        "recovery_id": recovery_event["payload"]["recovery_id"],
        "conversation_id": recovery_event["conversation_id"],
        "archive_id": recovery_event["archive_id"],
        "created_at": utc_now(),
        "journal_path": str(journal_path),
        "classification": classification,
        "prefix_event_count": replay["event_count"],
        "discarded_offset_bytes": prefix_length,
        "discarded_size_bytes": len(discarded),
        "discarded_sha256": sha256_bytes(discarded) if discarded else "",
        "recovered_journal_sha256": sha256_bytes(journal_path.read_bytes()),
        "replay_valid": replay["valid"],
        "errors": replay["errors"],
    }

    if receipt_path is not None:
        receipt_path.parent.mkdir(parents=True, exist_ok=True)
        receipt_path.write_text(
            json.dumps(receipt, indent=2, sort_keys=True), encoding="utf-8"
        )

    return {
        "valid": replay["valid"],
        "applied": True,
        "classification": classification,
        "prefix_event_count": replay["event_count"],
        "discarded_size_bytes": len(discarded),
        "errors": replay["errors"],
        "receipt": receipt,
    }


# ── journal writer ──


class ArchiveJournalWriter:
    """Append-only archive operation journal with hash-chain integrity."""

    def __init__(
        self,
        journal_path: Path,
        *,
        archive_id: str,
        conversation_id: str,
        last_sequence: int = 0,
        last_event_sha256: str | None = None,
    ) -> None:
        journal_path.parent.mkdir(parents=True, exist_ok=True)
        self._journal_path = journal_path
        self._archive_id = archive_id
        self._conversation_id = conversation_id
        self._sequence = last_sequence
        self._prev_hash = last_event_sha256
        self._handle: BinaryIO | None = None

    def __enter__(self) -> ArchiveJournalWriter:
        self._handle = self._journal_path.open("ab")
        return self

    def __exit__(self, *args: object) -> None:
        if self._handle is not None:
            self._handle.flush()
            os.fsync(self._handle.fileno())
            self._handle.close()
            self._handle = None

    @property
    def next_sequence(self) -> int:
        return self._sequence

    @property
    def last_event_sha256(self) -> str | None:
        return self._prev_hash

    def append_event(
        self,
        event_type: str,
        payload: dict[str, Any],
    ) -> dict[str, Any]:
        if self._handle is None:
            raise AssuranceError("ArchiveJournalWriter is not open")
        event = {
            "schema_version": "0.1.0-draft",
            "archive_id": self._archive_id,
            "sequence": self._sequence,
            "timestamp": utc_now(),
            "conversation_id": self._conversation_id,
            "event_type": event_type,
            "previous_event_sha256": (
                None if self._sequence == 0 else self._prev_hash
            ),
            "payload": payload,
            "payload_sha256": sha256_bytes(canonical_bytes(payload)),
            "event_sha256": "",
        }
        event["event_sha256"] = _event_hash(event)
        _validate_event(event, label=f"archive journal event {self._sequence}")
        self._handle.write(_canonical_json_line(event))
        self._handle.flush()
        os.fsync(self._handle.fileno())
        self._prev_hash = event["event_sha256"]
        self._sequence += 1
        return event


# ── stale lock detection ──


def detect_stale_archive_lock(
    journal_path: Path,
    *,
    max_age_seconds: float = 300.0,
) -> dict[str, Any]:
    """Detect orphaned archive lock files left behind after a crash.

    Checks whether a ``.lock`` file exists without an active lock holder.
    On Windows, attempts a non-blocking lock acquisition — if it succeeds
    immediately, the lock was stale.

    Returns a dict with keys:
      - stale: bool — whether the lock is stale
      - lock_path: str — path to the lock file
      - lock_exists: bool — whether a lock file exists
      - action: str — "cleanup", "wait", or "none"
      - age_seconds: float | None — file age in seconds (None if not found)
      - details: str — human-readable diagnostic
    """
    lock_path = _archive_lock_path(journal_path)

    if not lock_path.exists():
        return {
            "stale": False,
            "lock_path": str(lock_path),
            "lock_exists": False,
            "action": "none",
            "age_seconds": None,
            "details": "No lock file exists.",
        }

    # Check file age
    try:
        age_seconds = time.time() - lock_path.stat().st_mtime
    except OSError:
        age_seconds = 0.0

    # Try to acquire the lock — if it succeeds immediately, the lock is stale
    try:
        with lock_path.open("a+b") as handle:
            _ensure_lock_byte(handle)
            acquired = _try_lock(handle)
            if acquired:
                _unlock(handle)
                action = "cleanup"
                details = (
                    f"Lock file exists but was not held by any process. "
                    f"Age: {age_seconds:.1f}s. Safe to remove."
                )
                stale = True
            else:
                action = "wait"
                details = (
                    f"Lock file is actively held by another process. "
                    f"Age: {age_seconds:.1f}s. "
                    f"{'Consider manual intervention if age exceeds expected archive duration.' if age_seconds > max_age_seconds else 'Wait for the active archive to complete.'}"
                )
                stale = False
    except OSError as exc:
        stale = False
        action = "none"
        details = f"Could not probe lock file: {exc}"

    return {
        "stale": stale,
        "lock_path": str(lock_path),
        "lock_exists": True,
        "action": action,
        "age_seconds": age_seconds,
        "details": details,
    }


def cleanup_stale_archive_lock(journal_path: Path) -> dict[str, Any]:
    """Remove a stale archive lock file.

    Only removes the lock if it can be confirmed as stale (not held by
    another process). Uses :func:`detect_stale_archive_lock` first.

    Returns a dict with keys:
      - removed: bool
      - lock_path: str
      - error: str | None
    """
    detection = detect_stale_archive_lock(journal_path)

    if not detection["stale"]:
        return {
            "removed": False,
            "lock_path": detection["lock_path"],
            "error": (
                None
                if detection["lock_exists"]
                else "no lock file to remove"
            ),
        }

    lock_path = _archive_lock_path(journal_path)
    try:
        lock_path.unlink()
        return {
            "removed": True,
            "lock_path": str(lock_path),
            "error": None,
        }
    except OSError as exc:
        return {
            "removed": False,
            "lock_path": str(lock_path),
            "error": str(exc),
        }
