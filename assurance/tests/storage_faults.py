"""Fault-injection storage adapter for testing archive resilience.

Provides :class:`FaultInjectionStorageAdapter` — a wrapper around
:class:`LocalStorageAdapter` that can inject configurable failures:
transient/permanent delete errors, slow storage, and directory walk errors.

Used by archive recovery and controller tests to verify retry, crash
recovery, and failure handling without depending on real filesystem faults.
"""

from __future__ import annotations

import time
from pathlib import Path
from typing import Callable, Iterator

from assurance.storage_adapter import LocalStorageAdapter


class FaultInjectionStorageAdapter(LocalStorageAdapter):
    """Storage adapter with configurable fault injection for testing.

    All faults are opt-in — by default this behaves identically to
    ``LocalStorageAdapter``.
    """

    def __init__(
        self,
        *,
        transient_delete_failures: int = 0,
        permanent_delete_failures: set[Path] | None = None,
        delete_delay_seconds: float = 0.0,
        walk_errors: set[Path] | None = None,
        delete_file: Callable[[Path], None] | None = None,
    ) -> None:
        super().__init__(delete_file=delete_file)
        self._transient_remaining = transient_delete_failures
        self._transient_count = transient_delete_failures
        self._permanent = permanent_delete_failures or set()
        self._delete_delay = delete_delay_seconds
        self._walk_errors = walk_errors or set()
        self.delete_attempts: list[tuple[Path, bool]] = []

    # ── fault counters (reset between tests) ──

    def reset_transient_counter(self) -> None:
        """Reset the transient failure counter to the initial value."""
        self._transient_remaining = self._transient_count

    @property
    def transient_failures_remaining(self) -> int:
        return self._transient_remaining

    # ── delete_file with fault injection ──

    def delete_file(self, path: Path) -> None:
        """Delete a file, injecting faults per configuration."""
        if self._delete_delay > 0:
            time.sleep(self._delete_delay)

        # Permanent failure check
        if path in self._permanent:
            self.delete_attempts.append((path, False))
            raise OSError(f"synthetic permanent delete failure: {path}")

        # Transient failure check
        if self._transient_remaining > 0:
            self._transient_remaining -= 1
            self.delete_attempts.append((path, False))
            raise OSError(
                f"synthetic transient delete failure "
                f"({self._transient_remaining + 1} remaining): {path}"
            )

        # Success
        self.delete_attempts.append((path, True))
        super().delete_file(path)

    # ── walk_directory with fault injection ──

    def walk_directory(
        self, root: Path
    ) -> Iterator[tuple[Path, list[str], list[str]]]:
        """Walk a directory, injecting errors for configured paths."""
        if root in self._walk_errors:
            raise OSError(f"synthetic walk error: {root}")
        yield from super().walk_directory(root)
