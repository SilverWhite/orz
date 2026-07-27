from __future__ import annotations

import os
from pathlib import Path
from typing import Callable, Iterator, Protocol


DeleteFile = Callable[[Path], None]


class StorageAdapter(Protocol):
    """Abstract filesystem backend for archive deletion operations."""

    def delete_file(self, path: Path) -> None:
        """Delete a single file. Raises OSError on failure."""
        ...

    def delete_directory(self, path: Path) -> None:
        """Delete an empty directory. Raises OSError on failure."""
        ...

    def file_exists(self, path: Path) -> bool:
        """Return True if path exists and is a regular file."""
        ...

    def directory_exists(self, path: Path) -> bool:
        """Return True if path exists and is a directory."""
        ...

    def is_link_or_reparse(self, path: Path) -> bool:
        """Return True if path is a symlink, junction, or reparse point."""
        ...

    def walk_directory(
        self, root: Path
    ) -> Iterator[tuple[Path, list[str], list[str]]]:
        """Yield (current_dir, subdirs, filenames) tuples, similar to os.walk."""
        ...

    def file_size(self, path: Path) -> int:
        """Return file size in bytes."""
        ...

    def file_digest(self, path: Path) -> str:
        """Return SHA-256 hex digest of file contents."""
        ...

    def require_within(
        self, path: Path, root: Path, *, must_exist: bool
    ) -> Path:
        """Resolve path and verify it lies within root. Raises on escape."""
        ...


class LocalStorageAdapter:
    """Default storage adapter backed by the local filesystem."""

    def __init__(self, delete_file: DeleteFile | None = None) -> None:
        self._delete_file = delete_file or _default_delete_file

    def delete_file(self, path: Path) -> None:
        self._delete_file(path)

    def delete_directory(self, path: Path) -> None:
        path.rmdir()

    def file_exists(self, path: Path) -> bool:
        return path.is_file()

    def directory_exists(self, path: Path) -> bool:
        return path.is_dir()

    def is_link_or_reparse(self, path: Path) -> bool:
        from .utils import is_link_or_reparse as _check

        return _check(path)

    def walk_directory(
        self, root: Path
    ) -> Iterator[tuple[Path, list[str], list[str]]]:
        for current_root, dirs, names in os.walk(root, followlinks=False):
            yield Path(current_root), dirs, names

    def file_size(self, path: Path) -> int:
        return path.stat().st_size

    def file_digest(self, path: Path) -> str:
        from .utils import sha256_file

        return sha256_file(path)

    def require_within(
        self, path: Path, root: Path, *, must_exist: bool
    ) -> Path:
        from .utils import require_within as _check

        return _check(path, root, must_exist=must_exist)


def _default_delete_file(path: Path) -> None:
    path.unlink()
